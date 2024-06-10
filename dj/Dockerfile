FROM python:3
ENV PYTHONUNBUFFERED 1
ENV env prod
WORKDIR /app
ADD . /app
COPY ./requirements.txt /app/requirements.txt
EXPOSE 80
RUN pip install -r requirements.txt
ADD run.sh /
RUN chmod +x /run.sh
CMD ["/bin/bash", "run.sh"]
