"""
Preemptible execution of a given plan.
"""

import random
import threading
import time
from typing import Dict, List, Tuple
import pause  # type: ignore

# List of tuples of (timestamp, command) where
# the timestamp is in seconds since the epoch. The events
# are ordered from last to first so that we can pop() to
# get the soonest event from the end.
EVENTS: List[Tuple[float, Dict[int, int]]] = []
EVENTS_LOCK = threading.Lock()

asleep = True
beginning = True


def runner():
    global EVENTS, EVENTS_LOCK, asleep, beginning

    time.sleep(2)

    cur_state = {}

    # def ser_write(ch, val):
    #     nonlocal cur_state
    #     cur_state[ch] = val
    #     print(f'{ch}c{val}w')

    # for offset in [0, 7]:
    #     ser_write(1+offset, 0)
    #     ser_write(2+offset, 0)
    #     ser_write(3+offset, 0)
    #     ser_write(4+offset, 0)
    #     ser_write(5+offset, 0)
    #     ser_write(6+offset, 40)
    #     ser_write(7+offset, 0)

    while True:
        EVENTS_LOCK.acquire()
        timestamp = 0
        while timestamp < time.time():
            try:
                timestamp, wanted_state = EVENTS.pop()
                if not beginning:
                    break
            except IndexError:
                asleep = True
                time.sleep(0.15)
                # print('1c0w2c0w5c0w')
                # ser.flushOutput()
                EVENTS_LOCK.release()
                return
        beginning = False
        EVENTS_LOCK.release()
        cmd = wanted_state
        # for k, v in wanted_state.items():
        #     if cur_state.get(k, -1) != v:
        #         cmd += f'{k}c{v}w'
        #         cur_state[k] = v
        # if cmd == "":
        #     continue

        # rand_key = random.choice(list(cur_state.keys()))
        # tmp = f'{rand_key}c{cur_state[rand_key]}w'
        # if tmp not in cmd:
        #     cmd += tmp

        pause.until(timestamp)
        try:
            print('$', cmd)
            import sys
            sys.stdout.flush()
        except serial.serialutil.SerialException as e:
            print('\nError while writing:', e)


            # pause.seconds(0.25)
            # try:
            #     ser = serial.Serial("/dev/tty.usbmodem101", 115200)
            # except Exception:
            #     ser = serial.Serial("/dev/tty.usbmodem1101", 115200)
            pause.seconds(2)

        # ser.flushOutput()
        # print("\r\033[2K" + cmd, end="", flush=True)


def exec(plan):
    global EVENTS, EVENTS_LOCK, asleep, beginning
    plan.sort(key=lambda x: x[0]) # +0.015
    # plan = [(t-0.02, s) for t, s in plan]
    cutoff = time.time()+0.25
    plan = [(t, s) for t, s in plan if t > cutoff]
    EVENTS_LOCK.acquire()
    beginning = True
    EVENTS = plan[::-1]
    if asleep:
        asleep = False
        queue_thread = threading.Thread(target=runner, args=[])
        queue_thread.daemon = True
        queue_thread.start()
    EVENTS_LOCK.release()
