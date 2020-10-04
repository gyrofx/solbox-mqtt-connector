FROM python:3.8-slim

COPY ./requirements.txt /code/
RUN pip install -r /code/requirements.txt

COPY . /code
WORKDIR /code


CMD ["python", "solbox.py", "--log", "/data/solbox.log"]

