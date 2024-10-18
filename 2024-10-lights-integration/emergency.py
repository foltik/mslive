import random
import time
import exec
import effects
import track
import lights

effect = effects.chain_strobe_on(effects.random_effect())


while True:
  tempo = random.random() * 20 + 80 + 80
  beat_dur = 60 / tempo
  measures_per_section = 16

  section_dur = beat_dur*4*measures_per_section

  disco = lights.TwoSpinLights(0, 7)


  section = track.Section(
    # beats=[beat_dur * i for i in range(measures_per_section * 4)],
    bars=[track.Bar(
      beats=[beat_dur * i * 4 + beat_dur * j for j in range(4)],
      start=beat_dur * i * 4,
      half=beat_dur * (i+.5) * 4,
      end=beat_dur * (i+1) * 4,
    ) for i in range(measures_per_section)],
    start=0,
    end=section_dur,
  )

  # print(section)
  print(effect)
  print()

  effect = effect(disco, section)


  plan = disco.reset()
  now = time.time()
  out = [(e.time+now, e.state) for e in plan]
  print(out)
  exec.exec(out)

  time.sleep(section_dur*3/4)
