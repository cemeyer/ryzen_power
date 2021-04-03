# zen_power_fbsd
Toy Rust utility to print actual clocks and voltage for AMD Zen CPUs on
FreeBSD.

# Example

This is on an AMD Ryzen 5800x:
```
$ sudo zen_power_fbsd
4.85 GHz @ 1.26V
3.23 GHz @ 1.26V
3.23 GHz @ 1.26V
3.23 GHz @ 1.26V
3.23 GHz @ 1.26V
3.23 GHz @ 1.26V
4.85 GHz @ 1.26V
3.23 GHz @ 1.26V
dev.amdtemp.0.core0.sensor0: 34.5C
dev.jedec_dimm.0.temp: 29.8C
```

`amdtemp(4)` must be loaded (but should work on any Zen CPU).

`jedec_dimm(4)` requires some fiddly configuration and you probably don't have
it enabled.  This program is a toy; feel free to clone this repo and delete
that sysctl in your version.

# As a CLI system monitor

```
$ sudo pkg install gnu-watch
$ gnu-watch -n 1.0 sudo zen_power_fbsd
Every 1.0s: sudo zen_power_fbsd                   n: Sat Apr  3 10:57:03 2021
4.80 GHz @ 1.40V
4.80 GHz @ 1.40V
4.80 GHz @ 1.40V
4.80 GHz @ 1.40V
4.80 GHz @ 1.40V
4.80 GHz @ 1.40V
4.80 GHz @ 1.40V
3.20 GHz @ 1.40V
dev.amdtemp.0.core0.sensor0: 65.8C
dev.jedec_dimm.0.temp: 29.6C
```
