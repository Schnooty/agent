FROM ubuntu:18.04 as builder

ARG LINUX_RELEASE_TAR_URL

RUN echo Downloading Schnooty Agent tarfile from $LINUX_RELEASE_TAR_URL
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt-get -yq install curl
RUN curl -L $LINUX_RELEASE_TAR_URL -o agent_tar.tar
RUN ls -l
RUN pwd
RUN tar xf agent_tar.tar

FROM ubuntu:18.04

RUN apt-get update
RUN apt-get install -yq libssl-dev

COPY --from=builder /schnooty .

ENV API_KEY

CMD ["./schnooty", "--api-key", "${API_KEY}"]
