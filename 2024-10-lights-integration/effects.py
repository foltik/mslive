from curses.ascii import SP
from operator import truediv
import random
import track
import lights
from lights import SpinEnable, SpinColors


FLASH_ENABLED = True
UV_ENABLED = True

FLASH_CONST = 0.5

def chain_strobe_on(func):
    def inner(disco, section):
        nonlocal func
        if len(section.bars) < 4 or len(section.bars[0].beats) != 4:
            return chain_strobe_on(func(disco, section))

        white = random.random() < 0.85
        if white:
            disco.enable(SpinEnable.COLOR_WHITE)
        else:
            disco.enable(SpinEnable.COLOR_UV)
        accent_toggle = disco.white if white else disco.uv
        strobe_speed = 10 if white else 20
        accent_toggle(True)
        disco.color(SpinColors.NONE)
        disco.strobe(strobe_speed)
        disco.at(section.bars[0].beats[0])
        disco.rotate(120)
        disco.color(SpinColors.ALL)
        accent_toggle(False)
        disco.strobe(10)
        disco.at(section.bars[0].beats[2])
        disco.color(SpinColors.NONE)
        uv_suffix = random.random() < 0.6
        accent_toggle = disco.uv if uv_suffix else disco.white
        if uv_suffix and white:
            disco.enable(SpinEnable.UV)
        accent_toggle(True)
        disco.strobe(0)
        disco.at(section.bars[1].beats[0])
        accent_toggle(False)
        disco.at(section.bars[1].beats[0] +
                 (section.bars[1].end-section.bars[1].start)/16)
        sec = track.Section(
            section.bars[2:], section.bars[2].beats[0], section.end)
        out = func(disco, sec)
        if random.random() < 1:  # FIXME: should be 0.9
            return chain_strobe_on(out)
        else:
            return chain_strobe_off(out)
    return inner


def chain_strobe_off(func):
    def inner(disco, section):
        out = func(disco, section)
        if random.random() < 0.5:
            return chain_strobe_on(out)
        else:
            return chain_strobe_off(out)
    return inner


def wrap(func, *args):
    return lambda disco, section: func(disco, section, *args)


def effect_init():
    return jack_effect_main
    # return random_effect()


def jack_effect_main(jack: lights.Jack, section: track.Section):
    opt_strobe = ["CHASE"]
    opt_nonstrobe = ["RANDOM"]*5
    state = random.choice(opt_nonstrobe + opt_strobe)
    jack.at(section.start, state)
    if state in opt_strobe and len(section.bars) > 2:
        jack.at(section.bars[1].start, random.choice(opt_nonstrobe))

    for bar in section.bars[4::8]:
        jack.at(bar.start, random.choice(opt_nonstrobe + opt_strobe))
    return jack_effect_main

def random_effect():
    # return effect_speed_views
    # return effect_color_blink_pause
    # return effect_speed_accel
    # return effect_flash_spin
    # return effect_speed_views
    # return effect_speed_accel
    return random_effect_middle()
    # return random_effect_pregame()
    # return effect_flash_spin
    # return random_effect_middle()
    # return random_effect_start()
    # return effect_speed_accel


def random_effect_pregame():
    if random.random() < 0.5:
        return effect_flash_spin
    return effect_cycle_colors



def effect_cycle_colors(disco: lights.TwoSpinLights, section: track.Section):
    colors = [SpinColors.RED, SpinColors.YELLOW, SpinColors.RED, SpinColors.BLUE]
    disco.white(False)
    disco.color(colors[0])
    disco.strobe(0)
    disco.rotate(140)

    disco.enable(SpinEnable.COLOR)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            disco.color(colors[i % 4])
            disco.at(beat)

    return random_effect()

def random_effect_start():
    aa = random.random()
    # if aa < 0.02:
    #     return effect_dark
    # return effect_color_outage
    # if aa < 0.85:
    #     return effect_flash_spin
    # if aa < 0.45:
    #     return effect_color_blink_pattern
    # elif aa < 0.05:
    return effect_speed_accel
    # else:
    #     return effect_speed_views
    # elif aa < 0.95:
    # else:
        # return effect_color_outage
    # else:
    #     return effect_color_blink


def random_effect_middle():
    # return effect_color_blink_pattern
    # if random.random() < 0.05:
    # #     return effect_dark
    if random.random() < 0.10:
        return effect_new_alternate

    if random.random() < 0.08:
        return effect_toggle_strobe
    if random.random() < 0.20:
        if random.random() < 0.4:
            return effect_color_flash
        return effect_flash_spin
    # if random.random() < 0.2:
    #     if random.random() < 0.6:
    #         return effect_diff_flash_uv
    #     if random.random() < 0.4:
    #         return effect_same_flash
    #     else:
    #         return effect_diff_flash
    if random.random() < 0.4:
        if random.random() < 0.3:
            return effect_speed_views
        elif random.random() < 0.5:
            return effect_speed_accel
        return effect_color_blink_pattern
    if random.random() < 0.10:
        return effect_color_outage
    return effect_color_blink

def effect_new_alternate(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    disco.white(False)
    disco.color(colors[0])
    disco.strobe(0)
    disco.rotate(140)
    disco.enable(SpinEnable.COLOR)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.left.rotate(140)
                    disco.right.rotate(20)
                    disco.left.color(colors[0])
                    disco.right.color(colors[1])
                case 1:
                    disco.left.color(colors[0])
                    disco.right.color(SpinColors.NONE)
                    disco.left.rotate(20)
                    disco.right.rotate(140)
                case 2:
                    disco.left.rotate(20)
                    disco.right.rotate(140)
                    disco.left.color(colors[0])
                    disco.right.color(colors[1])
                case 3:
                    disco.left.color(SpinColors.NONE)
                    disco.right.color(colors[1])
                    disco.left.rotate(140)
                    disco.right.rotate(20)
            disco.at(beat)
    return random_effect()


def effect_color_blink(disco: lights.TwoSpinLights, section: track.Section):
    if random.random() < 0.4:
        return effect_color_blink_slow(disco, section)
    else:
        return effect_color_blink_fast(disco, section)

def effect_toggle_strobe(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    disco.enable(SpinEnable.COLOR)
    disco.white(False)
    disco.color(colors[0])
    disco.strobe(0)
    disco.rotate(140)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0 | 2:
                    disco.left.color(colors[0])
                    disco.right.color(colors[1])
                case 1 | 3:
                    disco.left.color(colors[1])
                    disco.right.color(colors[0])
            disco.at(beat)
    return random_effect()

def effect_color_blink_slow(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    disco.enable(SpinEnable.COLOR)
    disco.white(False)
    disco.color(colors[0])
    disco.strobe(0)
    disco.rotate(140)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.color(colors[0])
                case 2:
                    disco.color(colors[1])
                case 1 | 3:
                    disco.color(SpinColors.NONE)
            disco.at(beat)
    return random_effect()


def effect_color_blink_fast(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    disco.white(False)
    disco.color(colors[0])
    disco.strobe(0)
    disco.rotate(140)

    flash = random.random() < FLASH_CONST and FLASH_ENABLED
    if flash:
        disco.enable(SpinEnable.COLOR_WHITE)
    else:
        disco.enable(SpinEnable.COLOR)

    alt = random.random() < 0.5

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            flashing_light = disco.left if alt else disco.right
            if i == 0:
                flashing_light.white(True)

            disco.color(colors[0 if i < 2 else 1])
            disco.at(beat)

            if i == 0:
                flashing_light.white(False)
                flashing_light.at(beat+(bar.end-bar.start)/16)
                alt = not alt

            disco.color(SpinColors.NONE)
            disco.at(beat+(bar.end-bar.start)/8)
    return random_effect()


def effect_color_blink_pattern(disco: lights.TwoSpinLights, section: track.Section):
    tmp = random.choice([
        [SpinColors.RED_YELLOW, SpinColors.RED, SpinColors.YELLOW],
        [SpinColors.RED_BLUE, SpinColors.RED, SpinColors.BLUE],
        [SpinColors.YELLOW_BLUE, SpinColors.YELLOW, SpinColors.BLUE],
    ])
    two = tmp[0]
    one = random.choice(tmp[1:])

    disco.white(False)
    disco.strobe(0)
    disco.rotate(140)

    flash = random.random() < FLASH_CONST and FLASH_ENABLED
    if flash:
        disco.enable(SpinEnable.COLOR_WHITE)
    else:
        disco.enable(SpinEnable.COLOR)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.rotate(180)
                    disco.color(SpinColors.ALL)
                case 2:
                    disco.rotate(100)
                    disco.color(two)
                case _:
                    disco.rotate(70)
                    disco.color(one)
            disco.at(beat)
            disco.color(SpinColors.NONE)
            disco.at(beat+(bar.end-bar.start)/8)
    return random_effect()


def effect_color_blink_pause(disco: lights.TwoSpinLights, section: track.Section):
    col = random.choice(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE]
    )

    disco.enable(SpinEnable.COLOR)
    disco.rotate(10)
    disco.color(col)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 2:
                    disco.color(col)
                    disco.rotate(0)
                case 3:
                    disco.color(SpinColors.NONE)
                    disco.rotate(10)
                case _:
                    disco.color(col)
            disco.at(beat)
    return random_effect()


def effect_speed_accel(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.choice([
        [SpinColors.RED_YELLOW, SpinColors.RED, SpinColors.YELLOW],
        [SpinColors.RED_BLUE, SpinColors.RED, SpinColors.BLUE],
        [SpinColors.YELLOW_BLUE, SpinColors.YELLOW, SpinColors.BLUE],
    ])
    tmp = colors[2:]
    random.shuffle(tmp)
    colors[2:] = tmp

    flash = random.random() <FLASH_CONST  and FLASH_ENABLED

    if flash:
        disco.enable(SpinEnable.COLOR_WHITE)
    else:
        disco.enable(SpinEnable.COLOR)
    disco.white(False)
    disco.strobe(0)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.rotate(180)
                    disco.color(SpinColors.ALL)
                    disco.white(True)
                    disco.at(beat)
                    disco.white(False)
                    disco.at(beat+(bar.end-bar.start)/16)
                case 1:
                    disco.color(colors[0])
                    disco.rotate(60)
                    disco.at(beat)
                case 2:
                    disco.color(colors[1])
                    disco.rotate(30)
                    disco.at(beat)
                case 3:
                    disco.color(colors[2])
                    disco.rotate(10)
                    disco.at(beat)
    return random_effect()


def effect_speed_views(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)

    flash = random.random() < FLASH_CONST and FLASH_ENABLED

    if flash:
        disco.enable(SpinEnable.COLOR_WHITE)
    else:
        disco.enable(SpinEnable.COLOR)
    disco.white(False)
    disco.strobe(0)
    disco.rotate(10)

    alt = 0 if False else 255

    idx = 0

    for bar in section.bars:
        idx += 1
        speed = 30 if idx % 2 == 0 else 180
        for i, beat in enumerate(bar.beats):
            disco.rotate(10 if i % 2 == 0 else alt)
            disco.color(colors[i % 2])
            disco.at(beat)
            disco.color(SpinColors.NONE)
            disco.at(beat+(bar.end-bar.start)/8)
            # disco.rotate(30 if i % 2 != 0 else alt)
    return random_effect()


def effect_dark(disco: lights.TwoSpinLights, section: track.Section):
    return random_effect()


def effect_color_flash(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    disco.enable(SpinEnable.COLOR_WHITE)
    disco.white(False)
    disco.strobe(0)
    disco.rotate(40)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.color(colors[0])
                    disco.white(True)
                    disco.at(beat)
                    disco.white(False)
                    disco.at(beat+(bar.end-bar.start)/16)
                case 1:
                    disco.color(SpinColors.NONE)
                    disco.at(beat)
                case 2:
                    disco.color(colors[1])
                    disco.at(beat)
                case 3:
                    disco.color(SpinColors.NONE)
                    disco.at(beat)
    return random_effect()


def effect_same_flash(disco: lights.TwoSpinLights, section: track.Section):
    disco.enable(SpinEnable.WHITE)
    disco.white(False)
    disco.strobe(0)

    for bar in section.bars:
        disco.white(True)
        disco.at(bar.beats[0])
        disco.white(False)
        disco.at(bar.beats[0]+(bar.end-bar.start)/16)
    return random_effect()


def effect_flash_spin(disco: lights.TwoSpinLights, section: track.Section):
    colors = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    if random.random() < 0.7 and FLASH_ENABLED:
        disco.enable(SpinEnable.COLOR_WHITE)
    else:
        disco.enable(SpinEnable.COLOR)
    disco.white(False)
    disco.strobe(0)

    slow = False

    alt = random.random() < 0.5

    for bar in section.bars:
        if slow:
            disco.rotate(90)
            disco.color(colors[0])
        else:
            disco.rotate(10)
            disco.color(colors[1])
        disco.at(bar.beats[0])
        slow = not slow

        flashing_light = disco.right if alt else disco.left
        flashing_light.white(True)
        flashing_light.at(bar.beats[0])
        flashing_light.white(False)
        flashing_light.at(bar.beats[0]+(bar.end-bar.start)/16)
        alt = not alt
    return random_effect()


def effect_poly(disco: lights.TwoSpinLights, section: track.Section):
    disco.enable(SpinEnable.COLOR_WHITE)
    disco.white(False)
    disco.color(SpinColors.RED)
    disco.strobe(0)
    disco.rotate(40)

    # CYCLE = [SpinColors.YELLOW_BLUE, SpinColors.RED_BLUE, SpinColors.RED_YELLOW]
    CYCLE = [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE]
    cycle_idx = 0

    for bar in section.bars:
        bar_t = bar.beats[0]
        disco.color(CYCLE[cycle_idx % len(CYCLE)])
        # disco.white(True)
        disco.at(bar_t)
        # disco.white(False)
        # disco.at(bar_t+0.1)

        div = 3

        poly_int = (bar.end - bar.start)/div
        disco.color(SpinColors.NONE)
        disco.at(bar_t+poly_int*0.5)
        cycle_idx += 1
        for i in range(1, div):
            disco.color(CYCLE[cycle_idx % len(CYCLE)])
            disco.at(bar.start+poly_int*i)
            disco.color(SpinColors.NONE)
            disco.at(bar.start+poly_int*(i+0.5))
            cycle_idx += 1
    return random_effect()


def effect_diff_flash(disco: lights.TwoSpinLights, section: track.Section):
    disco.enable(SpinEnable.WHITE)
    disco.white(False)
    disco.strobe(0)
    disco.rotate(40)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.left.white(True)
                    disco.left.at(beat)
                    disco.left.white(False)
                    disco.left.at(beat+0.1)
                case 2:
                    disco.right.white(True)
                    disco.right.at(beat)
                    disco.right.white(False)
                    disco.right.at(beat+0.1)
                case _:
                    pass
    return random_effect()


def effect_diff_flash_uv(disco: lights.TwoSpinLights, section: track.Section):
    disco.enable(SpinEnable.WHITE)
    disco.white(False)
    disco.strobe(0)
    disco.rotate(40)

    def impl(start, dur, a, b):
        b.enable(SpinEnable.UV)
        b.uv(False)
        b.at(start)
        if dur > 0.05:
            b.uv(True)
            b.at(start+dur*4)
            b.uv(False)
            b.at(start+dur*5)

        a.enable(SpinEnable.WHITE)
        a.white(True)
        a.at(start)
        a.white(False)
        a.enable(SpinEnable.UV)
        a.uv(True)
        a.at(start+dur)

    for bar in section.bars:
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    impl(beat, (bar.end-bar.start)/16, disco.left, disco.right)
                case 2:
                    impl(beat, (bar.end-bar.start)/16, disco.right, disco.left)
                case _:
                    pass
    return random_effect()


def effect_color_outage(disco: lights.TwoSpinLights, section: track.Section):
    CYCLE = random.sample(
        [SpinColors.RED, SpinColors.YELLOW, SpinColors.BLUE], 2)
    cycle_idx = 0
    disco.enable(SpinEnable.COLOR)
    disco.white(False)
    disco.color(CYCLE[0])
    disco.strobe(0)
    disco.rotate(40)

    for bar in section.bars:
        cycle_idx += 1
        bar_color = CYCLE[cycle_idx % len(CYCLE)]
        for i, beat in enumerate(bar.beats):
            match i % 4:
                case 0:
                    disco.color(bar_color)
                    disco.at(beat)
                case 3:
                    disco.color(SpinColors.NONE)
                    disco.at(beat+0*(bar.end-bar.start)/16)
                case _:
                    pass
    return random_effect()
