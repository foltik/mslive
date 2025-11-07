#!/bin/bash

#sudo ifconfig en9 10.16.4.2 netmask 255.255.252.0

HOST=stagebridge
#HOST=10.16.4.3
SSH="ssh -i ~/.ssh/id_ed25519_stagebridge"

export SSH_AUTH_SOCK=
rsync -e "$SSH" -Pavzr src/ $HOST:~/mslive/src
$SSH -t $HOST '. ~/.cargo/env && cd mslive && cargo build --release && pkill stagebridge'
