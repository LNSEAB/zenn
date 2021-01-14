FROM alpine:latest

RUN apk update && \
	apk upgrade && \
	apk add --no-cache nodejs npm git

RUN mkdir /mnt/zenn
WORKDIR /mnt/zenn

RUN npm init --yes && npm install zenn-cli
RUN npx zenn init

