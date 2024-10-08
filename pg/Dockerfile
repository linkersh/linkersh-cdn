FROM postgres:16.2-bullseye
RUN apt-get update -y && apt-get -y install postgresql-16-cron postgresql-16-postgis-3
COPY /pg/pg_hba.conf /var/lib/postgresql/data/pg_hba.conf
