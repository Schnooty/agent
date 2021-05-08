FROM ubuntu:18.04 as builder

ARG LINUX_RELEASE_TAR_URL

# Retrieve the Schnooty Agent assets

RUN echo Downloading Schnooty Agent tarfile from $LINUX_RELEASE_TAR_URL
RUN apt-get update
RUN DEBIAN_FRONTEND=noninteractive apt-get -yq install curl
RUN curl -L $LINUX_RELEASE_TAR_URL -o agent_tar.tar
RUN tar xf agent_tar.tar

# Build the actual image

FROM ubuntu:18.04

# Install SSL stuff
RUN apt-get update
RUN apt-get install -yq libssl-dev
RUN apt-get install -yq ca-certificates

COPY --from=builder schnooty schnooty

CMD /schnooty --api-key $API_KEY
