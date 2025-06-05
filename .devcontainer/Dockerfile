FROM ubuntu:latest

RUN apt-get update && apt-get install -y flex bison build-essential csh libxaw7-dev wget curl
Run curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

RUN mkdir -p /usr/class && cd /usr/class \
    && wget -O student-dist.tar.gz https://courses.edx.org/asset-v1:StanfordOnline+SOE.YCSCS1+1T2020+type@asset+block@student-dist.tar.gz \
    && tar -xf student-dist.tar.gz && rm student-dist.tar.gz

ENV PATH=/usr/class/bin:$PATH

WORKDIR /usr/class/
