# Coffee order process

Customer -> Cafe:Menu
The customer reads the menu to choose their coffee.

Customer -> Cafe:Cashier
The customer places their order and pays for the coffee.

Cafe:Cashier -> Cafe:Barista
The cashier relays the coffee order to the barista.

Cafe:Barista -> Cafe:Grinder
The barista measures and grinds the coffee beans for the espresso shot.

Cafe:Grinder -> Cafe:Espresso Machine:Portafilter
The ground coffee is placed into the espresso machine's portafilter.

Cafe:Espresso Machine:Portafilter -> Cafe:Espresso Machine
The barista attaches the portafilter to the espresso machine.

Cafe:Espresso Machine -> Cafe:Espresso Machine:Cup
The espresso machine brews the coffee, dispensing it into a cup.

Cafe:Barista -> Cafe:Refrigerator
If the order includes milk, the barista retrieves milk from the refrigerator.

Cafe:Refrigerator -> Cafe:Espresso Machine:Steam Wand
The milk is steamed using the espresso machine's steam wand for lattes or cappuccinos.

Cafe:Espresso Machine:Steam Wand -> Cafe:Espresso Machine:Cup
The steamed milk is poured into the cup with the espresso, creating the coffee drink.

Cafe:Espresso Machine:Cup -> Cafe:Barista
The barista picks up the finished coffee drink.

Cafe:Barista -> Customer
The barista serves the coffee drink to the customer.
