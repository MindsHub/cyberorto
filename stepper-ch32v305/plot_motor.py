import matplotlib.pyplot as plt

with open("tmp") as f:
    lines = f.readlines()

lines = [line.split(" ")[1:] for line in lines if line.startswith("TRACE") and not "value" in line]
xs = list(range(len(lines)))
position = [float(line[0]) for line in lines]
objective = [float(line[1]) for line in lines]
output = [float(line[2]) for line in lines]
counter_deriv = [0] + [float(lines[i][3])-float(lines[i-1][3]) for i in range(1,len(lines))]

plt.plot(xs, position, label="position")
plt.plot(xs, objective, label="objective")
plt.plot(xs, output, label="output")
plt.plot(xs, counter_deriv, label="counterderiv")

plt.show()