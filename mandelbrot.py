"""Toy program which computes the Mandelbrot set over a region of
the complex plane.

Intended as a benchmark compute-intensive program.
"""
import numpy as np
import matplotlib.pyplot as plt

# Resolution of the image
WIDTH, HEIGHT = (400, 300)
MAX_STEPS = 50
DIVERGENCE_LIMIT = 4


@np.vectorize
def mandelbrot(c):
    z = 0 + 0j
    for num_steps in range(MAX_STEPS):
        z = z ** 2 + c
        num_steps += 1
        if abs(z) >= DIVERGENCE_LIMIT:
            return num_steps
    return MAX_STEPS


if __name__ == '__main__':
    xs = np.linspace(-2, 1, WIDTH)
    ys = np.linspace(-1, 1, HEIGHT)

    xx, yy = np.meshgrid(xs, ys)
    zz = mandelbrot(xx + 1j * yy)

    plt.imshow(zz)
    plt.show()
