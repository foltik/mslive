import json
import os
import pathlib
import subprocess
import sys
import time
from typing import Any, Dict, List
import plan

import requests
from dmx import DiscoEnable

import lights
import spotipy  # type: ignore
import track
import effects
from spotipy.oauth2 import SpotifyOAuth  # type: ignore
import exec

sp = spotipy.Spotify(
    auth_manager=SpotifyOAuth(
        scope="user-read-currently-playing",
        client_id="34945b66c6df4b1cac2eb025bc226348",
        client_secret="bd7981c754b24a0bad6b451aee66e81d",
        redirect_uri='https://ajanse.me/',
    )
)


def watch():
    """Watches for current spotify song, pushing a plan to the event queue."""
    prev_id = None
    prev_seek_start = 0
    DURATION = 3
    while True:
        try:
            current = sp.currently_playing()
            # print(current)
        except Exception as e:
            print("\r\033[2KConnection Error:", e)
            time.sleep(10)
            continue

        if current is None or current["item"] is None or not current["is_playing"]:
            print('NONE')
            if prev_id is not None:
                exec.exec([])
            prev_id = None
            time.sleep(DURATION)
            continue

        current_id = current["item"]["id"]
        changed = current_id != prev_id
        # cur_seek_start = get_seek_start()
        cur_seek_start =( current['timestamp']-current['progress_ms'])/1000
        print("Started n secs ago", time.time() - cur_seek_start)
        if abs(cur_seek_start - prev_seek_start) > 1:
            changed = True
        prev_seek_start = cur_seek_start
        if changed:
            if current_id != prev_id:
                print("\r\033[2K\nPlaying:", current["item"]["name"])
                print("ID:", current_id)
            prev_id = current_id
            handle_change(current_id, current["item"])

        time.sleep(DURATION)


def get_seek_start():
    tmp = subprocess.run(
        [
            "osascript",
            "-e",
            'tell application "Spotify" to log (player position as text)',
        ],
        stderr=subprocess.PIPE,
    ).stderr
    return time.time() - float(tmp.decode("utf-8").strip())



def handle_change(track_id, track):
    if track_id is None:
        exec.exec([])
        return

    try:
        # seek_start = get_seek_start()
        seek_start = 0
        current = sp.currently_playing()
        seek_start =( current['timestamp']-current['progress_ms'])/1000
        plan_ = plan.plan(track_id, seek_start, sp)
    except Exception as e:
        print("\r\033[2KError during planning:", e)
        plan_ = []
        raise e

    try:
        exec.exec(plan_)
    except Exception as e:
        print("\r\033[2KError during execution:", e)


if __name__ == "__main__":
    print("\033[?25l~~ Welcome to Discofy! ~~")

    try:
        watch()
    except KeyboardInterrupt:
        print("\r\033[2K\033[?25hExiting.")
        sys.exit(1)



# current = sp.currently_playing()
# track_id = current['item']['id']

# osa_t = time.time()
# osa_res = subprocess.run([
#     "osascript",
#     "-e",
#     'tell application \"Spotify\" to log (player position as text)',
# ], stderr=subprocess.PIPE).stderr
# current_seek = float(osa_res.decode('utf-8').strip())
# start_seek = osa_t - current_seek + 0.02




# def get_analysis(track_id):
#     cache = pathlib.Path("./analysis")
#     try:
#         with open(cache / track_id, "r") as f:
#             return json.load(f)
#     except IOError:
#         analysis = sp.audio_analysis(track_id)
#         with open(cache / track_id, "x") as f:
#             json.dump(analysis, f)
#         return analysis

# analysis = get_analysis(track_id)

# disco = lights.OneSpinLight(0)

# # t = time.time() + 3

# # disco.enable(lights.SpinEnable.WHITE)
# # disco.white(True)
# # disco.strobe(10)
# # disco.at(t)
# # t += 0.2
# # disco.strobe(11)
# # disco.at(t)
# # t += 0.2
# # disco.strobe(12)
# # disco.at(t)
# # t += 0.8
# # disco.strobe(30)
# # disco.at(t)
# # t += 2
# # disco.white(False)
# # # disco.strobe(0)
# # disco.at(t)
# # disco.strobe(0)
# # disco.at(t+1)

# # disco.enable(lights.SpinEnable.WHITE)
# # disco.white(True)
# # disco.at(10)

# effect = effects.effect_init()

# for section in analysis['sections']:
#     s_start: float = section['start']
#     s_end: float = s_start + section['duration']
#     bars: List[track.Bar] = []
#     for bar in analysis['bars']:
#         b_start = bar['start']
#         b_end = b_start + bar['duration']
#         if not (s_start <= b_start < s_end-0.001):
#             continue
#         beats: List[float] = []
#         for beat in analysis['beats']:
#             t = beat['start']
#             if not (b_start <= t < b_end-0.001):
#                 continue
#             beats.append(t)
#         print(beats)
#         bars.append(track.Bar(beats, b_start, b_start+b_end, b_end))
#     section_obj = track.Section(bars, s_start, s_end)

#     effect = effect(disco, section_obj)

# plan = disco.reset()

# print(plan)
# # queue = [(e.time, e.state) for e in plan]
# queue = [(start_seek+e.time, e.state) for e in plan]


# exec.exec(queue)

# time.sleep(100)

# queue: List[Any] = []
# state: Dict[int, int] = {}
# for event in plan:
#     cmd = ''
#     for k, v in event.state.items():
#         print(state.get(k, -1), v)
#         if state.get(k, -1) != v:
#             cmd += f'{k}c{v}w'
#             state[k] = v
#     if cmd == '':
#         continue
#     # cmd = '11c80w9c50w'
#     if len(queue) > 0 and event.time == queue[-1][0]:
#         queue[-1][1] += cmd
#     else:
#         queue.append([start_seek+event.time, cmd])

# print(queue)
# print(requests.post('http://raspberrypi:8080', json=queue))

# print(plan)
# track.Section


# def flash_sync_at(start, dest):
#     out = []
#     t = dest
#     while t > start:
#         out.append((t, '4c100w2c100w'))
#         out.append((t+0.1, f'2c0w4c200w'))
#         t -= 0.4
#     t = dest
#     while t > start:
#         out.append((t, '11c100w9c100w'))
#         out.append((t+0.1, f'9c0w11c200w'))
#         t -= 0.44444

#     # t = dest
#     # while t < dest + 3:
#     #     out.append((t, '11c240w9c100w4c240w2c100w'))
#     #     out.append((t+0.1, f'9c0w11c200w2c0w4c200w'))
#     #     t += 0.3

#     out.sort(key=lambda x: x[0])
#     return out


# def flash_stretch_at(start, dest):
#     # out = []
#     # t = dest
#     # while t > start:
#     #     out.append((t, '4c100w2c100w'))
#     #     out.append((t+0.1, f'2c0w4c200w'))
#     #     t -= 0.4
#     # t = dest
#     # while t > start:
#     #     out.append((t, '11c100w9c100w'))
#     #     out.append((t+0.1, f'9c0w11c200w'))
#     #     t -= 0.44444

#     out = []

#     t = start
#     skip = 0.1
#     while t < (dest-start)/2:
#         out.append((t, '11c240w9c100w4c240w2c100w'))
#         out.append((t+0.1, f'9c0w11c200w2c0w4c200w'))
#         t += skip
#         skip += 0.05

#     skip = 0.1
#     while t < dest:
#         out.append((t, '11c240w9c100w4c240w2c100w'))
#         out.append((t+0.1, f'9c0w11c200w2c0w4c200w'))
#         t += skip
#         skip = skip/2 + 0.05
#         # skip -= 0.05

#     out.sort(key=lambda x: x[0])
#     return out

# # start_t = time.time()
# # dest_t = start_t + 5
# # plan = flash_sync_at(start_t, dest_t)

# # requests.post('http://raspberrypi:8080', json=plan)

# # remaining = lambda: dest_t - time.time()
# # debug = lambda: print(f'\r\x1b[2KRemaining: {round(remaining())}', end='')
# # debug()
# # pause.seconds(remaining() % 1)
# # while remaining() > 1:
# #     debug()
# #     pause.seconds(1)

# # debug()
# # pause.seconds(remaining())

# # print('\narrived!')

# # plan = []

# # for i, beat in enumerate(analysis['beats']):
# #     t = start_seek+beat['start']
# #     if i % 4 == 0:
# #         plan.append((t, '4c100w2c100w'))
# #         plan.append((t+0.1, f'2c0w4c200w'))
# #     elif i % 4 == 2:
# #         plan.append((t, '11c100w9c100w'))
# #         plan.append((t+0.1, f'9c0w11c200w'))

# # plan.sort(key=lambda x: x[0])

# # for section in analysis['sections']:
# #     goal = section['start']+start_seek
# #     plan.extend(flash_stretch_at(goal, goal + 10))
# #     pass
# #     remaining = lambda: start_seek + section['start'] - time.time()
# #     debug = lambda: print(f'\r\x1b[2KRemaining: {round(remaining())}', end='')
# #     if remaining() < 0:
# #         continue
# #     debug()
# #     pause.seconds(remaining() % 1)
# #     while remaining() > 1:
# #         debug()
# #         pause.seconds(1)

# #     debug()
# #     pause.seconds(remaining())

# #     print('\narrived!')
# #     # print(section)

# # for section in analysis['sections']:
# #     dist = min(((bar['start'], abs(section['start']-bar['start']) )for bar in analysis['bars']), key=lambda x: x[1])
# #     print(dist[0])

# # t = (num-1)/90+0.55

# # t = 60 / analysis['track']['tempo'] * 2
# # code = 90*(t-0.55)

# # print(round(code))

# # num =

# # plan = plan2.effect(track_id, start_seek, sp)
# # plan = [(t+0.05, cmd) for (t, cmd) in plan]
# # requests.post('http://raspberrypi:8080', json=plan)
