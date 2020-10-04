FROM python:3.8-slim

COPY ./requirements.txt /code/
RUN pip install -r /code/requirements.txt

COPY . /code
WORKDIR /code

ENV PYTHONUNBUFFERED 1


CMD ["python", "solbox.py"]

