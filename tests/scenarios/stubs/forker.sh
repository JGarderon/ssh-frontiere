#!/bin/sh
for i in 1 2 3 4 5; do
    sleep 60 &
done
wait
