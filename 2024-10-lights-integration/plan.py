import json
import pathlib
from typing import List
import lights
import effects
import track

effect = effects.effect_init()

def plan(track_id, seek_start, sp):
    global effect

    analysis = get_analysis(track_id, sp)

    tempo = analysis['track']['tempo']
    print('$ TEMPO', tempo)
    import sys
    sys.stdout.flush()
    # if tempo > 150:
    #     analysis['bars'] = analysis['bars'][::2]
    #     for b in analysis['bars']:
    #         b['duration'] *= 2
    #     analysis['beats'] = analysis['beats'][::2]
    # print(analysis['track']['tempo'])

    # disco = lights.TwoSpinLights(0, 7)
    jack = lights.Jack()

    for section in analysis['sections']:
        s_start: float = section['start']
        s_end: float = s_start + section['duration']
        bars: List[track.Bar] = []
        for bar in analysis['bars']:
            b_start = bar['start']
            b_end = b_start + bar['duration']
            if not (s_start <= b_start < s_end-0.001):
                continue
            beats: List[float] = []
            for beat in analysis['beats']:
                t = beat['start']
                if not (b_start <= t < b_end-0.001):
                    continue
                beats.append(t)
            bars.append(track.Bar(beats, b_start, b_start+b_end, b_end))
        if len(bars) < 2:
            continue

        section_obj = track.Section(bars, s_start, s_end)

        effect = effect(jack, section_obj)

    plan = jack.reset()

    queue = [(seek_start+e.time, e.state) for e in plan]
    return queue

def get_analysis(track_id, sp):
    cache = pathlib.Path("./analysis")
    try:
        with open(cache / track_id, "r") as f:
            return json.load(f)
    except IOError:
        analysis = sp.audio_analysis(track_id)
        with open(cache / track_id, "x") as f:
            json.dump(analysis, f)
        return analysis
