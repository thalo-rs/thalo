#!/bin/bash

build() {
  docker build -t awto-es:latest ./src
}

push() {
  docker push acidic9/awto-es:latest
}

help() {
  echo "  possible arguments:"
  echo "    build"
  echo "    push"
}

if [ -z "$1" ]
  then
    echo "no argument specified"
    help
    exit 1
fi

if [ $1 = "build" ]; then
  build
elif [ $1 = "push" ]; then
  push
else
  echo "invalid argument"
  help
  exit 1
fi
