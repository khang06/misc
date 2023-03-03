from z3 import *
import time
import math
import sys

# Not faster
# set_param('parallel.enable', True)


def op_to_str(input_count: int, op: int):
    if op < input_count:
        return f"in{op}"
    else:
        return f"r{op - input_count}"


def try_solve(gates: int, input_count: int, truth_table: dict):
    start = time.time()
    output_count = len(list(truth_table.values())[0])

    # Create a variable for each input for each path
    s = Solver()
    input = [[Bool(f"input_{x}_{y}") for x in range(input_count)]
             for y in range(len(truth_table))]

    # Create input and output variables for each gate
    # The inputs are numbers as they are indices into circuit inputs or past outputs
    # For example, for a two input problem with 3 gates:
    # 0: in0, 1: in1, 2: r0, 3: r1: 4: r2
    # Using a BitVec is significantly faster than using Int
    bits = math.ceil(math.log2(input_count + gates)) + 1
    a = [BitVec(f"a_{x}", bits) for x in range(gates)]
    b = [BitVec(f"b_{x}", bits) for x in range(gates)]
    r = [[Bool(f"r_{x}_{y}") for x in range(gates)]
         for y in range(len(truth_table))]

    # Create output constraints
    output = [BitVec(f"output_{x}", bits) for x in range(output_count)]
    for x in output:
        # Negative indices are invalid
        s.add(x >= 0)

        # Only a limited number of possible circuit input or gate output references
        s.add(x < input_count + gates)

    # Create gate constraints
    for i in range(gates):
        # Negative indices are invalid
        s.add(a[i] >= 0)
        s.add(b[i] >= 0)

        # Prevent an input from referencing the output of its own or any future gate
        # This heavily reduces the search space, but also prevents the solver from finding a solution to "Latch"
        s.add(a[i] < input_count + i)
        s.add(b[i] < input_count + i)

        # Generate gate expressions for each path
        # The gate input indices are solved globally, but additional constraints are needed per path
        # This will constrain the per-path gate outputs to read the inputs specified by the input indices and perform a NAND
        for j in range(len(truth_table)):
            a_expr = False
            b_expr = False
            for k in range(input_count):
                a_expr = If(a[i] == k, input[j][k], a_expr)
                b_expr = If(b[i] == k, input[j][k], b_expr)
            for k in range(i):
                a_expr = If(a[i] == k + input_count, r[j][k], a_expr)
                b_expr = If(b[i] == k + input_count, r[j][k], b_expr)
            s.add(r[j][i] == Not(And(a_expr, b_expr)))

    for i, (k, v) in enumerate(truth_table.items()):
        # Set the path inputs to the ones from the truth table
        for j in range(input_count):
            s.add(input[i][j] == k[j])

        # Create the output expressions
        # The outputs act exactly like the gate input indices, except their outputs are constrained per-path according to the truth table
        for j in range(output_count):
            output_expr = False
            for k in range(input_count):
                output_expr = If(output[j] == k, input[i][k], output_expr)
            for k in range(gates):
                output_expr = If(output[j] == k +
                                 input_count, r[i][k], output_expr)
            s.add(output_expr == v[j])

    # Used to test other SMT solvers when Z3 is too slow
    with open("cur.smt2", "w") as smt2:
        smt2.write("(set-logic QF_BV)")
        smt2.write("(set-option :produce-models true)")
        smt2.write(s.sexpr())
        smt2.write("(check-sat)")
        smt2.write("(get-model)")

    # Run the solver
    check = s.check()
    print(f"{check} in {time.time() - start}s")
    if check == sat:
        model = s.model()
        for i in range(gates):
            print(
                f"r{i}: nand({op_to_str(input_count, model[a[i]].as_long())}, {op_to_str(input_count, model[b[i]].as_long())})")
        for i in range(output_count):
            print(
                f"output {i}: {op_to_str(input_count, model[output[i]].as_long())}")
        return True
    elif check == unknown:
        # Probably a CTRL+C
        return True
    return False


# Invert
# Optimal solution is 1 gate
# r0: nand(in0, in0)
# output 0: r0
'''
TRUTH_TABLE = {
    (False,): [True],
    (True,): [False],
}
'''

# And
# Optimal solution is 2 gates
# r0: nand(in1, in0)
# r1: nand(r0, r0)
# output 0: r1
'''
TRUTH_TABLE = {
    (False, False): [False],
    (False, True): [False],
    (True, False): [False],
    (True, True): [True],
}
'''

# Or
# Optimal solution is 3 gates
# r0: nand(in0, in0)
# r1: nand(in1, in1)
# r2: nand(r1, r0)
# output 0: r2
'''
TRUTH_TABLE = {
    (False, False): [False],
    (False, True): [True],
    (True, False): [True],
    (True, True): [True],
}
'''

# Xor
# Optimal solution is 4 gates
# r0: nand(in1, in0)
# r1: nand(in1, r0)
# r2: nand(in0, r0)
# r3: nand(r1, r2)
# output 0: r3
'''
TRUTH_TABLE = {
    (False, False): [False],
    (False, True): [True],
    (True, False): [True],
    (True, True): [False],
}
'''

# Half adder
# Optimal solution is 5 gates
# r0: nand(in1, in0)
# r1: nand(r0, r0)
# r2: nand(r0, in1)
# r3: nand(r0, in0)
# r4: nand(r3, r2)
# output 0: r1
# output 1: r4
'''
TRUTH_TABLE = {
    (False, False): [False, False],
    (False, True): [False, True],
    (True, False): [False, True],
    (True, True): [True, False],
}
'''

# Full adder
# Optimal solution is 9 gates
# r0: nand(in0, in1)
# r1: nand(in0, r0)
# r2: nand(in1, r0)
# r3: nand(r2, r1)
# r4: nand(r3, in2)
# r5: nand(r4, r0)
# r6: nand(in2, r4)
# r7: nand(r4, r3)
# r8: nand(r6, r7)
# output 0: r5
# output 1: r8
TRUTH_TABLE = {
    (False, False, False): [False, False],
    (False, False, True): [False, True],
    (False, True, False): [False, True],
    (False, True, True): [True, False],
    (True, False, False): [False, True],
    (True, False, True): [True, False],
    (True, True, False): [True, False],
    (True, True, True): [True, True],
}

# Multi-bit Adder
# 17-gate solution couldn't be found via Z3/Bitwuzla/CVC5 within 1.5 hours
# However, 1 to 13-gate solutions are proven to be impossible via the solver
# Best human solution is 18 gates (2x half adder)
'''
TRUTH_TABLE = {}
for a in range(4):
    for b in range(4):
        for c in range(2):
            sum = a + b + c
            TRUTH_TABLE[(a & 2 == 2, a & 1 == 1, b & 2 == 2, b & 1 == 1, c == 1)] = [
                sum & 4 == 4, sum & 2 == 2, sum & 1 == 1]
'''

# Selector
# Optimal solution is 4 gates
# r0: nand(in0, in0)
# r1: nand(in0, in1)
# r2: nand(r0, in2)
# r3: nand(r2, r1)
# output 0: r3
'''
TRUTH_TABLE = {
    (False, False, False): [False],
    (False, True, False): [False],
    (False, False, True): [True],
    (False, True, True): [True],
    (True, False, False): [False],
    (True, False, True): [False],
    (True, True, False): [True],
    (True, True, True): [True],
}
'''

# Switch
# Optimal solution is 4 gates
# r0: nand(in0, in1)
# r1: nand(r0, r0)
# r2: nand(r0, in1)
# r3: nand(r2, r2)
# output 0: r1
# output 1: r3
'''
TRUTH_TABLE = {
    (False, False): [False, False],
    (False, True): [False, True],
    (True, False): [False, False],
    (True, True): [True, False],
}
'''

input_count = len(list(TRUTH_TABLE.keys())[0])
gates = 1
while True:
    print(f"trying to solve with {gates} gates...", end=" ")
    sys.stdout.flush()
    if try_solve(gates, input_count, TRUTH_TABLE):
        break
    gates += 1
