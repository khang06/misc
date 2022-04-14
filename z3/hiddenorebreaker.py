from z3 import *
from time import *
from struct import *
from binascii import *
from functools import *

# This is an unfinished proof of concept for breaking the HiddenOre plugin, abusing the fact that it uses Math.random()
# This wouldn't actually work very well in a real server scenario because Math.random() still gets called for some stuff like entity spawning
# Even in a perfect environment with no other players and no entities spawning,
# there's still the missing piece of being able to leak the global Math.random() state to manipulate it in the first place

rand_state = 0x5DEECE66D

def java_rand_setseed(seed):
    global rand_state
    rand_state = seed ^ 0x5DEECE66D

def java_rand_next(bits):
    global rand_state
    rand_state = (rand_state * 0x5DEECE66D + 0xB) & ((1 << 48) - 1)
    ret = rand_state >> (48 - bits)
    return ret

def java_rand_nextlong():
    return (java_rand_next(32) << 32) + java_rand_next(32)

def java_rand_nextdouble():
    return ((java_rand_next(26) << 27) + java_rand_next(27)) / float(1 << 53)

def abs(x):
    return If(x >= 0,x,-x)

def double_to_hex(f):
    return hex(unpack('<Q', pack('<d', f))[0])

if __name__ == '__main__':
    print("doing it")
    start = time()
    count = 7

    s = SimpleSolver()

    # don't care about the initial seed value, only need the current state to predict the rest...
    # 2 states are used in each double
    states = [z3.BitVec('state_' + str(i), 64) for i in range(count * 2)]
    for i in range(1, count * 2):
        s.add(states[i] == ((0x5DEECE66D * states[i - 1] + 0xB) & ((1 << 48) - 1)))
    s.add(states[1] != rand_state)
    #for i in range(count):
        #s.add((fpUnsignedToFP(RTZ(), (((states[2 * i] >> 22) << 27) + (states[2 * i + 1] >> 21)), Float64()) / 9007199254740992.0) < 0.5)
        #s.add((fpUnsignedToFP(RTZ(), (((states[2 * i] >> 22) << 27) + (states[2 * i + 1] >> 21)), Float64())) < 9007199254740992.0 * 0.5)
        #s.add(((states[2 * i] >> 22) << 27) + (states[2 * i + 1] >> 21) < int(9007199254740992.0 * 0.01))
    #s.add(((states[0] >> 22) << 27) + (states[1] >> 21) > int(9007199254740992.0 * 0.5))

    # not really sure what the dice order is
    PROB_OFFSET = 0.000597 + 0.001437 + 0.01 + 0.01025 # lapis + gold + coal + redstone
    PROB_AMOUNT = 0.0005427 # diamond probability

    s.add(((states[2] >> 22) << 27) + (states[3] >> 21) > int(9007199254740992.0 * PROB_OFFSET))
    s.add(((states[2] >> 22) << 27) + (states[3] >> 21) < int(9007199254740992.0 * (PROB_OFFSET + PROB_AMOUNT)))

    # attempt to maximize drop
    s.add(((states[4] >> 22) << 27) + (states[5] >> 21) > int(9007199254740992.0 * (2.0 / 3.0)))
    
    # discard a random state since it's called in item constructor and is useless
    # also gets incremented by one too?

    s.add(((states[9] >> 22) << 27) + (states[10] >> 21) > int(9007199254740992.0 * PROB_OFFSET))
    s.add(((states[9] >> 22) << 27) + (states[10] >> 21) < int(9007199254740992.0 * (PROB_OFFSET + PROB_AMOUNT)))

    # attempt to maximize drop
    s.add(((states[11] >> 22) << 27) + (states[12] >> 21) > int(9007199254740992.0 * (2.0 / 3.0)))

    solutions = 0
    while True:
        if s.check() == sat:
            print('satisfied in ' + str(time() - start))
            #print(s.model())
            java_rand_setseed(s.model()[states[1]].as_long() ^ 0x5DEECE66D)
            print(rand_state ^ 0x5DEECE66D)
            #print(s.model()[states[1]].as_long())
            #for i in range(count):
                #print(java_rand_nextdouble())
            s.add(states[1] != rand_state)
            solutions += 1
        else:
            print(f"{solutions} solutions")
            break