For example:

> cargo run --release -p dwarf-stats -- dwarf-stats/testcases/symbolicator.debug

# gigantic:

- https://github.com/electron/electron/releases/download/v13.6.1/electron-v13.6.1-linux-x64-debug.zip

DWARF size: 1.5G

```
Total executable bytes in sections: 105_251_358
Total address range covered: 108_348_699 (coverage: 102%)
Number of ranges: 12_325_128
Median range: 7
p90 range: 17
p99 range: 41
p998 range: 68
```

# huge:

- https://github.com/getsentry/symbolicator/releases/download/0.4.0/symbolicator-Linux-x86_64-debug.zip

DWARF size: 406M

```
Total executable bytes in sections: 12_811_220
Total address range covered: 22_144_266 (coverage: 172%)
Number of ranges: 2_024_783
Median range: 7
p90 range: 22
p99 range: 67
p998 range: 134
```

- https://github.com/getsentry/relay/releases/download/21.10.0/relay-Linux-x86_64-debug.zip

DWARF size: 408M

```
Total executable bytes in sections: 14_279_473
Total address range covered: 23_270_433 (coverage: 162%)
Number of ranges: 2_213_339
Median range: 6
p90 range: 22
p99 range: 61
p998 range: 125
```
