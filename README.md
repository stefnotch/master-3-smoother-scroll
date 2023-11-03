# master-3-smoother-scroll

The Logitech MX Master 3S has a smooth scrolling feature.
However, it's hyper-sensitive and will slightly scroll even when you're seemingly not touching the wheel.

This fixes that by analyzing the scroll events and only sending them to the OS if they're above a certain (speed) threshold.

Demo videos at https://github.com/stefnotch/master-3-smoother-scroll/issues/2

## Rewrite

1. Dead reckoning
   (aka ignore small scrolls, useful for "click on scroll wheel without accidentally scrolling")

- absolute_position = (0, timestamp) // Absolute position of the scroll wheel
- every scroll wheel event adds/subtracts from absolute_position
- if absolute_position is above a certain threshold, then it becomes spicy

2. Origin recentering
   (aka ignore slow drift over time)

- recenter = time_delta _ speed _ -sign(absolute_position) // How much we recenter the origin of the scroll wheel
- < code does stuff >
- absolute_position = absolute_position - recenter // We recenter the origin of the scroll wheel

3. Above threshold

- send a scroll event to the OS

what if I _predict_ the scroll (aka overscroll) and then when the speed drops to low, I "eat" the scroll events in order to compensate for the overscroll?
