from datetime import datetime
import json
import logging
import requests
import re
import time
from requests.exceptions import HTTPError
import urllib3
import argparse
import os
from persistqueue import Queue, Empty


logger = logging.getLogger(__name__)


urllib3.disable_warnings()

logger.info('Starting up Solbox logger')

SOREL_BASE_URL = 'https://db7aec.sorel-connect.net'
SOREL_LOGIN_URL = f'{SOREL_BASE_URL}/nabto/hosted_plugin/login/execute'
SOREL_SENSOR_URL = f'{SOREL_BASE_URL}/sensors.json'
SOREL_RELAYS_URL = f'{SOREL_BASE_URL}/relays.json'

SOREL_COOKIE_NAME = 'nabto-session'
SOREL_USERNAME = os.environ.get('SOLBOX_USERNAME', 'felix.eichenberger@gmail.com')
SOREL_PASSWORD = os.environ.get('SOLBOX_PASSWORD', 'UiPLnHhSdjn6gd')

ENERGIE_MON_BASE_URL = os.environ.get('ENERGIE_MON_BASE_URL', 'http://localhost:9000')
ENERGIE_MON_TOKEN = os.environ.get('ENERGIE_MON_TOKEN', '0a8a518add87b3e0524105cade3c22e5f609f5ea')


SENSOR_BOILER_TEMP_ABOVE_ID = 3
SENSOR_BOILER_TEMP_BELOW_ID = 2
SENSOR_COLLECTOR_TEMP_ID = 1
PUMP_STATE_ID = 1

SERIES_BOILER_TEMP_ABOVE = 'temp_boiler_oben'
SERIES_BOILER_TEMP_BELOW = 'temp_boiler_unten'
SERIES_COLLECTOR_TEMP = 'temp_kollektor'
SERIES_PUMP_STATE = 'pumpe_status'


parser = argparse.ArgumentParser(description='Log Solbox\'s sensors')
parser.add_argument('--log', help='Log path')

args = parser.parse_args()


class TokenAuth(requests.auth.AuthBase):

    def __init__(self, token=None):
        self.token = token

    def __call__(self, r):
        r.headers['Authorization'] = 'Token ' + self.token
        return r


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
    return json.loads(r.content.decode('utf-8'))


def get_series():
    response = requests.get(
        f'{ENERGIE_MON_BASE_URL}/api/v1/data/series/', auth=auth)
    response.raise_for_status()
    return response.json()


def find_series(series, name):
    return [s for s in series if s['name'] == name][0]


def send_value(series_id, time: datetime, value):
    response = requests.post(f'{ENERGIE_MON_BASE_URL}/api/v1/data/datapoint/',
                             auth=auth,
                             data={
                                 'time': time.isoformat(),
                                 'data': value,
                                 'series': series_id,
                             })
    response.raise_for_status()

def send_values(values, queue):
    for item in values:
        try:
            send_value(*item)
        except Exception as e:
            queue.put(item)
            logger.error(f'Failed to send value:\n{e}')


def get_values_from_queue(queue):
    queued_items = []
    try:
        while item := queue.get(block=False):
            queued_items.append(item)
            if len(queued_items) > 100:
                break
    except Empty:
        pass

    if queued_items:
        logger.info(f'get {len(queued_items)} item from queue')

    return queued_items


def process():
    q = Queue("/data/queue.dat", autosave=True)

    now = datetime.now()

    res_pump = get_relay_value(PUMP_STATE_ID, session)
    res_pump = 100 if res_pump['val'] else 0

    values = (
        (series_boiler_collector['id'], now, int(get_sensor_value(SENSOR_COLLECTOR_TEMP_ID, session)['val'])),
        (series_boiler_above['id'], now, int(get_sensor_value(SENSOR_BOILER_TEMP_ABOVE_ID, session)['val'])),
        (series_boiler_below['id'], now, int(get_sensor_value(SENSOR_BOILER_TEMP_BELOW_ID, session)['val'])),
        (series_pumpe_state['id'], now, res_pump),
    )

    send_values(values, q)
    values = get_values_from_queue(q)
    send_values(values, q)

try:

    session = connect(SOREL_USERNAME, SOREL_PASSWORD)
    auth = TokenAuth(token=ENERGIE_MON_TOKEN)

    series = get_series()
    series_boiler_above = find_series(series, SERIES_BOILER_TEMP_ABOVE)
    series_boiler_below = find_series(series, SERIES_BOILER_TEMP_BELOW)
    series_boiler_collector = find_series(series, SERIES_COLLECTOR_TEMP)
    series_pumpe_state = find_series(series, SERIES_PUMP_STATE)

    while(True):
        process()
        time.sleep(60)

except HTTPError as e:
    print(f"Request failed\n{e}\n{e.response.text}")