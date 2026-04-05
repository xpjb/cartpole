# Race Mode
initial race mode fitness function of x position - local minimum - model just drives right as fast as possible at the start

Needs survival time (residual gradient)

new fitness function:

fitness_fail = W_TIME * time_s + W_DIST * max_x
fitness_success = BASE_SUCCESS + W_SPEED / time_s


alright its not bad
i want to try quadratic model
i would change reward function a bit too but still


better fitness function
would take away time as it went but reward distacne squared i think


i kind of think its underfitting, might try quadratic control

---

wow quadratic is cool
is there like taylor series control lol 

anyway now its pretty good but like i feel could be better at learning
maybe better population management, survive good solutions, fitness weighting them as well
against homog
could make100 copies of best one and then mutate for example, lol

mutation hyperparameters etc

gonna try bernoulli trials
bernoulli weight and like mutation amount could be parameters too lol