a = 5
b = 3
c = 7
p = 15
q = 4
r = 6
s = 2
a1 = 5
b1 = 3
c1 = 7
p1 = 15
q1 = 4
r1 = 6
s1 = 2
s15 = s16 = 1
res = (
    (a * (b - c))
    % ((p % q) * (r + s))
    * (a1 * (b1 - c1))
    % ((p1 % q1) * (r1 + s1) * s15 * s16)
)
assert res == 3
