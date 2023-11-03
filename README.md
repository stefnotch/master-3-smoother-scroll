# master-3-smoother-scroll

The Logitech MX Master 3S has a smooth scrolling feature.
However, it's hyper-sensitive and will slightly scroll even when you're seemingly not touching the wheel.

This fixes that by analyzing the scroll events and only sending them to the OS if they're above a certain (speed) threshold.
