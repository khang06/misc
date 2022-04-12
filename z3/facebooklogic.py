from z3 import *

s = Solver()

digit1, digit2, digit3 = Ints("digit1 digit2 digit3")

# every digit must be between 0 and 9
s.add(digit1 >= 0, digit1 <= 9)
s.add(digit2 >= 0, digit2 <= 9)
s.add(digit3 >= 0, digit3 <= 9)

# 682 -> one number is correct and well placed
s.add(Or(
    digit1 == 6,
    digit2 == 8,
    digit3 == 2
))

# 614 -> one number is correct but wrong place
s.add(Or(
    digit2 == 6,
    digit3 == 6,
    digit1 == 1,
    digit3 == 1,
    digit1 == 4,
    digit2 == 4
))

# 206 -> two numbers are correct but wrong places
s.add(
    Or(
        # 2 and 0 correct
        And(
            Or(
                digit2 == 2,
                digit3 == 2
            ),
            Or(
                digit1 == 0,
                digit3 == 0
            )
        ),
        # 0 and 6 correct
        And(
            Or(
                digit1 == 0,
                digit3 == 0
            ),
            Or(
                digit1 == 6,
                digit2 == 6
            )
        ),
        # 2 and 6 correct
        And(
            Or(
                digit2 == 2,
                digit3 == 2
            ),
            Or(
                digit1 == 6,
                digit2 == 6
            )
        )
    )
)

# 738 -> nothing is correct
s.add(
    digit1 != 7,
    digit2 != 7,
    digit3 != 7,
    digit1 != 3,
    digit2 != 3,
    digit3 != 3,
    digit1 != 8,
    digit2 != 8,
    digit3 != 8
)

# 780 -> one number is correct but wrong place
s.add(Or(
    digit2 == 7,
    digit3 == 7,
    digit1 == 8,
    digit3 == 8,
    digit1 == 0,
    digit1 == 0
))

while True:
    result = s.check()
    if result == sat:
        #print("sat")
        model = s.model()
        print(f"{model[digit1]}{model[digit2]}{model[digit3]}")
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