#!/usr/bin/env bash
sudo ifconfig en13 down
sudo ifconfig en13 up
sudo ifconfig en13 10.16.4.2 netmask 255.255.252.0
