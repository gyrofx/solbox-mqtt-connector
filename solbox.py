from datetime import datetime
import json
import requests
import re
import time
from requests.exceptions import HTTPError
import urllib3
import argparse
import os
import paho.mqtt.client as mqtt

print("SOLBOX connector")

urllib3.disable_warnings()


SOREL_BASE_URL = 'https://db7aec.sorel-connect.net'
SOREL_LOGIN_URL = f'{SOREL_BASE_URL}/nabto/hosted_plugin/login/execute'
SOREL_SENSOR_URL = f'{SOREL_BASE_URL}/sensors.json'
SOREL_RELAYS_URL = f'{SOREL_BASE_URL}/relays.json'

SOREL_COOKIE_NAME = 'nabto-session'
SOREL_USERNAME = os.environ.get('SOLBOX_USERNAME', 'felix.eichenberger@gmail.com')
SOREL_PASSWORD = os.environ.get('SOLBOX_PASSWORD', 'UiPLnHhSdjn6gd')

DEFAULT_MQTT_BROKER_HOST = '192.168.110.50'
DEFAULT_MQTT_BROKER_PORT = 1883

SENSOR_BOILER_TEMP_ABOVE_ID = 3
SENSOR_BOILER_TEMP_BELOW_ID = 2
SENSOR_COLLECTOR_TEMP_ID = 1
PUMP_STATE_ID = 1

TOPIC_BOILER_TEMP_ABOVE = 'rehalp/solbox/boiler/sensors/temperature-top'
TOPIC_BOILER_TEMP_BELOW = 'rehalp/solbox/boiler/sensors/temperature-bottom'
TOPIC_COLLECTOR_TEMP = 'rehalp/solbox/collector/sensors/temperature'
TOPIC_PUMP_STATE = 'rehalp/solbox/pump/status'


parser = argparse.ArgumentParser(description='Log Solbox\'s sensors')
parser.add_argument('--log', help='Log path')

args = parser.parse_args()


# The callback for when the client receives a CONNACK response from the server.
def on_connect(client, userdata, flags, rc):
    print("Connected with result code "+str(rc))

    # Subscribing in on_connect() means that if we lose the connection and
    # reconnect then subscriptions will be renewed.
    # client.subscribe("$SYS/#")

def on_disconnect(client, userdata, rc):
   print("client disconnected ok")

def on_message(client, userdata, msg):
    print(msg.topic+" "+str(msg.payload))

def on_publish(client, userdata, msg):
    print(f"on_puhlish:  {msg}")

def on_log(client, userdata, level, buf):
    print("log: ",buf)

mqttc = mqtt.Client()
mqttc.on_connect = on_connect
mqttc.on_disconnect = on_disconnect

mqtt_host = os.environ.get('MQTT_BROKER_HOST', DEFAULT_MQTT_BROKER_HOST)
mqtt_port = os.environ.get('MQTT_BROKER_PORT', DEFAULT_MQTT_BROKER_PORT)

mqttc.connect(mqtt_host, mqtt_port)

def connect(username: str, password: str):
    url = f'{SOREL_LOGIN_URL}?email={username}&password={password}'
    r = requests.post(url, verify=False)
    if r.status_code != 200:
        raise Exception(f"Failed to connect to {url}")
    return r.cookies[SOREL_COOKIE_NAME]


def get_sensor_value(sensor_id, session):
    url = f'{SOREL_SENSOR_URL}?id={sensor_id}'
    data = get_value(url, sensor_id, session)
    match = re.match('(\d+)(.*)', data['response']['val'])

    return {'val': match.group(1), 'unit': match.group(2)}


def get_relay_value(sensor_id, session):
    url = f'{SOREL_RELAYS_URL}?id={sensor_id}'
    data = get_value(url, sensor_id, session)
    match = re.match('(\d+)_(.*)', data['response']['val'])

    if match.group(2) == 'ON':
        val = True
    elif match.group(2) == 'OFF':
        val = False
    else:
        val = match.group(2)
    return {'val': val}


def get_value(url, sensor_id, session):
    cookies = {SOREL_COOKIE_NAME: session}

    r = requests.get(url, cookies=cookies, verify=False, )
    r.raise_for_status()
    return json.loads(r.content.decode('utf-8'))


def send_value(topic, time: datetime, value):
    mqttc.publish(topic, payload=json.dumps({
        'value': value,
        'time': time.isoformat(),
    }), qos=2)

    
def send_values(values):
    for item in values:
        try:
            send_value(*item)
        except Exception as e:
            print(f'Error: Failed to send value:\n{e}')


def process():
    print('Process')
    
    now = datetime.now()

    res_pump = get_relay_value(PUMP_STATE_ID, session)
    res_pump = 100 if res_pump['val'] else 0

    values = (
        (TOPIC_COLLECTOR_TEMP, now, int(get_sensor_value(SENSOR_COLLECTOR_TEMP_ID, session)['val'])),
        (TOPIC_BOILER_TEMP_ABOVE, now, int(get_sensor_value(SENSOR_BOILER_TEMP_ABOVE_ID, session)['val'])),
        (TOPIC_BOILER_TEMP_BELOW, now, int(get_sensor_value(SENSOR_BOILER_TEMP_BELOW_ID, session)['val'])),
        (TOPIC_PUMP_STATE, now, res_pump),
    )
    print(f'values: {values}')

    send_values(values)

try:
    print('startup')
    session = connect(SOREL_USERNAME, SOREL_PASSWORD)

    mqttc.loop_start()

    while(True):
        process()
        time.sleep(60)
    

except HTTPError as e:
    print(f"Request failed\n{e}\n{e.response.text}")

finally:
    mqttc.loop_stop()