from z3 import *

s = Solver()

# define all digits
digits = [Int(f"digit_{i}") for i in range(10)]

# ensure that every digit is from 0 to 9
s.add([And(x >= 0, x <= 9) for x in digits])

# list of values given by the problem
constraints = [
    (8809, 6),
    (7111, 0),
    (2172, 0),
    (6666, 4),
    (1111, 0),
    (3213, 0),
    (7662, 2),
    (9313, 1),
    (0000, 4),
    (2222, 0),
    (3333, 0),
    (5555, 0),
    (8193, 3),
    (8096, 5),
    (1012, 1),
    (7777, 0),
    (9999, 4),
    (7756, 1),
    (6855, 3),
    (9881, 5),
    (5531, 0)
]

for x, y in constraints:
    s.add(
        digits[x // 1000 % 10] +
        digits[x // 100 % 10] + 
        digits[x // 10 % 10] +
        digits[x % 10] == y
    )

while True:
    result = s.check()
    if result == sat:
        print("sat")
        model = s.model()
        #print(model)
        print(f"2581 = {model[digits[2]] + model[digits[5]] + model[digits[8]] + model[digits[1]]}")
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