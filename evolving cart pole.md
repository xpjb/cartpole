# Cart Pole Balancer Evolutionary Experiment

## Software
* Rust
* Macroquad
* Headless mode (runs as fast as possible) and graphical mode
* fixed timestep: 60hz
* Save progress (Save a file with the current best population in working directory)
* whether in headless or graphical. graphical we watch the runs in real time. headless it goes as fast as possible

## Physics
* Euler
Here is exactly how you translate that into your Rust simulation.1. The Constants ("The Stuff")You will need to define these physical properties in your simulation:$M$: Mass of the cart (e.g., $1.0$ kg)$m$: Mass of the pole (e.g., $0.1$ kg)$l$: Distance to the pole's center of mass (this is exactly half the pole's total length, e.g., $0.5$ m)$g$: Gravity (e.g., $9.81$ m/s$^2$)$F$: The force applied to the cart (this is the output of your 4-term linear controller)2. The Equations of MotionAt every step of your simulation, you need to calculate the current acceleration of the cart ($\ddot{x}$) and the angular acceleration of the pole ($\ddot{\theta}$).First, calculate the pole's angular acceleration ($\ddot{\theta}$):$$\ddot{\theta} = \frac{g \sin\theta + \cos\theta \left( \frac{-F - m l \dot{\theta}^2 \sin\theta}{M + m} \right)}{l \left( \frac{4}{3} - \frac{m \cos^2\theta}{M + m} \right)}$$Once you have $\ddot{\theta}$, you plug it into the equation for the cart's acceleration ($\ddot{x}$):$$\ddot{x} = \frac{F + m l (\dot{\theta}^2 \sin\theta - \ddot{\theta} \cos\theta)}{M + m}$$Note: These equations assume a frictionless environment. If you want to add track friction or hinge friction later, you would add those dampening terms into the numerators.

* add a small amount of noise to the pole angle each timestep. try to just use rand for this i guess

* semi-implicit euler, update velocities first then update positions

## Evolution
* hyperparameters as constants
* population size... 100
* There are 4 simulation parameters, also 4 variables. so 4 things to learn: the coefficients of those variables into the control input
* crossover randomly each 4 thing among winning members
* mutation.
* criteria is how long until pole touches the ground or a wall
