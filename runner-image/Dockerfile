FROM python:3.12
ENV PYTHONUNBUFFERED 1
RUN apt update
RUN apt install postgresql-client -y
ENV env prod
WORKDIR /app
COPY . /app
EXPOSE 80
RUN pip install -r requirements.txt
RUN chmod +x run.sh
CMD ["/bin/bash", "run.sh"]
