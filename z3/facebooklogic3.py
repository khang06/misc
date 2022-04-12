from z3 import *

s = Solver()

# define variables
cookie = Int("cookie")
milk = Int("milk")
cookie_milk = Int("cookie_milk")

# define constraints
s.add(cookie * 3 == 36)
s.add(cookie_milk + milk + cookie_milk == 6)
s.add(cookie + cookie + cookie_milk == 24)

while True:
    result = s.check()
    if result == sat:
        print("sat")
        model = s.model()
        print(model)
        block = []
        for x in model:
            c = x()
            block.append(c != model[x])
        s.add(Or(block))
    elif result == unsat:
        #print("unsat")
        print("finished")
        break
    elif result == unknown:
        print("unknown")
        break