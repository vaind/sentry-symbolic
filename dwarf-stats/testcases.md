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
p999 range: 90

Number of files: 36_154
Median #lines: 274
p90 #lines: 1943
p99 #lines: 11651
p999 #lines: 142_620
```

- https://symbols.mozilla.org/libxul.so/9967F68DC87796379ACAF78D5225D9B60/libxul.so.dbg.gz

DWARF size: 2.2G

```
Total executable bytes in sections: 84_226_614
Total address range covered: 81_195_873 (coverage: 96%)
Number of ranges: 8_345_577
Median range: 7
p90 range: 18
p99 range: 47
p999 range: 120

Number of files: 17_687
Median #lines: 389
p90 #lines: 2134
p99 #lines: 9938
p999 #lines: 47_035
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
p999 range: 161

Number of files: 2323
Median #lines: 267
p90 #lines: 1910
p99 #lines: 5004
p999 #lines: 22_739
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
p999 range: 150

Number of files: 2994
Median #lines: 256
p90 #lines: 1749
p99 #lines: 3581
p999 #lines: 7073
```

# medium

- https://packages.debian.org/bullseye/amd64/libc6-dbg/download
  /usr/lib/debug/.build-id/54/eef5ce96cf37cb175b0d93186836ca1caf470c.debug

DWARF size: 3.5M

```
Total executable bytes in sections: 1_352_965
Total address range covered: 1_320_516 (coverage: 97%)
Number of ranges: 139_594
Median range: 5
p90 range: 19
p99 range: 56
p999 range: 167

Number of files: 1519
Median #lines: 452
p90 #lines: 1944
p99 #lines: 3767
p999 #lines: 5195
```
