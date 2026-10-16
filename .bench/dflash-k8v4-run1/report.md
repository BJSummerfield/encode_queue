# Run report: dflash-k8v4-run1

Server: spec=dflash2 kv=k8v4 instance=serve-425-1792174104492422 (up 0 min at preflight)

Total wall clock (preflight → last ledger event): 12264 s

## Requests by phase (ninfer per-request records, exact)

| phase | reqs | out tok | prefilled tok | decode tok/s p50 / p10 | ttft s p50 / p90 | queue s p90 | spec accept % | preemptions | paused s | replayed tok | non-stop finishes |
|---|---|---|---|---|---|---|---|---|---|---|---|
| P0 (2 s) | 1 | 254 | 197 | 252.42 / 252.42 | 0.11 / 0.11 | 0 | 45.2 | 0 | 0 | 0 | 0 |
| P1 (819 s) | 405 | 288082 | 343738 | 183.43 / 150.1 | 0.23 / 0.85 | 0.28 | 38.2 | 0 | 0 | 0 | 1 |
| P2 (1035 s) | 176 | 196162 | 364062 | 220.99 / 178.17 | 0.17 / 0.89 | 0.06 | 36.4 | 0 | 0 | 0 | 0 |
| P3 (333 s) | 132 | 129359 | 281855 | 195.48 / 155.55 | 0.19 / 1.29 | 0.1 | 40.1 | 0 | 0 | 0 | 0 |
| P4 (1034 s) | 116 | 197905 | 282767 | 219.86 / 176.74 | 0.16 / 0.68 | 0.03 | 36.7 | 0 | 0 | 0 | 0 |
| P5 (7040 s) | 1928 | 1810767 | 3434082 | 200.83 / 153.05 | 0.18 / 0.7 | 0.05 | 35.8 | 2 | 9 | 0 | 0 |
| P6 (5 s) | 2 | 471 | 7955 | 175.28 / 175.28 | 0.19 / 0.19 | 0.07 | 45.3 | 0 | 0 | 0 | 0 |
| P7 (1964 s) | 426 | 435766 | 1867781 | 179.43 / 145.25 | 0.76 / 8.43 | 5.67 | 34.1 | 9 | 98 | 73738 | 0 |
| P8 (20 s) | 3 | 1280 | 781 | 210.5 / 209.49 | 0.17 / 0.17 | 0.01 | 40.5 | 0 | 0 | 0 | 0 |
| **run** | 3193 | 3061489 | 6583772 | 197.04 / 152.53 | 0.2 / 1.49 | 0.15 | 36.1 | 11 | 106 | 73738 | 1 |

## RAM tier and scheduling (15-minute tiles; per-phase rows are approximate, the run row is exact)

| phase | tiles | spill pages | RAM restores | KV h2d GiB | KV h2d s | peak host KV GiB | preemptions | paused s | peak paused |
|---|---|---|---|---|---|---|---|---|---|
| P0 | 1 | 7620 | 18 | 1.2 | 0.17 | 9.58 | 0 | 0 | 0 |
| P1 | 1 | 7620 | 18 | 1.2 | 0.17 | 9.58 | 0 | 0 | 0 |
| P2 | 3 | 19323 | 23 | 1.2 | 0.17 | 14.47 | 0 | 0 | 0 |
| P3 | 1 | 5726 | 1 | 0 | 0 | 14.47 | 0 | 0 | 0 |
| P4 | 2 | 10619 | 1 | 0 | 0 | 14.8 | 0 | 0 | 0 |
| P5 | 9 | 139730 | 41 | 141.16 | 14.47 | 15.74 | 7 | 79 | 1 |
| P6 | 1 | 86318 | 14 | 131.78 | 13.09 | 15.74 | 6 | 73 | 1 |
| P7 | 3 | 176720 | 36 | 249.2 | 24.05 | 15.74 | 10 | 100 | 2 |
| P8 | 1 | 4843 | 0 | 0.44 | 0.05 | 15.45 | 0 | 0 | 0 |
| **run** | 14 | 249455 | 86 | 259.78 | 25.6 | 15.74 | 11 | 106 | 2 |

## Tasks

| task | role | round | wall s | outcome | out tok | turns | tool calls |
|---|---|---|---|---|---|---|---|
| P1.R1 | worker | 1 | 123 | done | 11960 | 21 | 21 |
| P1.R2 | worker | 1 | 335 | done | 41404 | 73 | 75 |
| P1.R3 | worker | 1 | 189 | done | 24895 | 65 | 32 |
| P1.R1 | reviewer | 1 | 262 | PASS | 31299 | 28 | 36 |
| P1.R4 | worker | 1 | 269 | done | 22103 | 32 | 34 |
| P1.R3 | reviewer | 1 | 257 | PASS | 30811 | 40 | 48 |
| P1.R2 | reviewer | 1 | 234 | PASS | 32319 | 54 | 57 |
| P1.R4 | reviewer | 1 | 210 | FAIL | 31240 | 32 | 35 |
| P1.R4 | worker | 2 | 32 | done | 4529 | 9 | 13 |
| P1.R4 | reviewer | 2 | 122 | PASS | 22376 | 28 | 31 |
| P2.C1 | worker | 1 | 178 | done | 30078 | 10 | 12 |
| P2.C1 | reviewer | 1 | 175 | FAIL | 30641 | 26 | 36 |
| P2.C1 | worker | 2 | 75 | done | 12407 | 16 | 18 |
| P2.C1 | reviewer | 2 | 287 | FAIL | 50252 | 34 | 38 |
| P2.C1 | worker | 3 | 122 | done | 23167 | 19 | 20 |
| P2.C1 | reviewer | 3 | 193 | PASS | 34023 | 34 | 37 |
| P3.A1 | worker | 1 | 89 | done | 8915 | 12 | 14 |
| P3.A2 | worker | 1 | 115 | done | 12034 | 15 | 18 |
| P3.A3 | worker | 1 | 105 | done | 11474 | 9 | 11 |
| P3.A4 | worker | 1 | 137 | done | 13443 | 12 | 13 |
| P3.A4 | reviewer | 1 | 101 | PASS | 16714 | 10 | 17 |
| P4.P1 | worker | 1 | 303 | done | 43387 | 21 | 25 |
| P4.P1 | reviewer | 1 | 276 | FAIL | 51756 | 23 | 31 |
| P4.P1 | worker | 2 | 275 | done | 46736 | 28 | 29 |
| P4.P1 | reviewer | 2 | 251 | PASS | 45151 | 20 | 26 |
| P5.WP01 | worker | 1 | 193 | done | 30636 | 34 | 52 |
| P5.WP01 | reviewer | 1 | 168 | FAIL | 30230 | 31 | 37 |
| P5.WP01 | worker | 2 | 33 | done | 4177 | 13 | 16 |
| P5.WP01 | reviewer | 2 | 92 | PASS | 18246 | 26 | 32 |
| P5.WP02 | worker | 1 | 191 | done | 21372 | 17 | 22 |
| P5.WP03 | worker | 1 | 236 | done | 29583 | 32 | 43 |
| P5.WP07 | worker | 1 | 121 | done | 14043 | 19 | 20 |
| P5.WP07 | reviewer | 1 | 235 | FAIL | 29372 | 28 | 36 |
| P5.WP02 | reviewer | 1 | 214 | PASS | 25938 | 20 | 29 |
| P5.WP03 | reviewer | 1 | 198 | PASS | 22601 | 31 | 34 |
| P5.WP07 | worker | 2 | 148 | done | 15296 | 28 | 30 |
| P5.WP04 | worker | 1 | 247 | done | 19780 | 27 | 32 |
| P5.WP05 | worker | 1 | 233 | done | 21496 | 32 | 43 |
| P5.WP07 | reviewer | 2 | 307 | done |  |  |  |
| P5.WP05 | reviewer | 1 | 378 | done | 35863 | 21 | 52 |
| P5.WP07-fix | worker | 1 | 56 | done | 5230 | 11 | 14 |
| P6.H | worker | 1 | 0 | done |  |  |  |
| P6.H | reviewer | 1 | 0 | PASS |  |  |  |
| P6.F2 | worker | 1 | 0 | done |  |  |  |
| P6.F2 | reviewer | 1 | 0 | PASS |  |  |  |
| P6.F1 | worker | 1 | 0 | done |  |  |  |
| P6.F1 | reviewer | 1 | 0 | PASS |  |  |  |
| P6.F3 | worker | 1 | 0 | done |  |  |  |
| P6.F3 | reviewer | 1 | 0 | PASS |  |  |  |
| P6.F4 | worker | 1 | 0 | done |  |  |  |
| P6.F4 | reviewer | 1 | 0 | PASS |  |  |  |
| P6.Z | reviewer | 1 | 0 | PASS |  |  |  |
| P7.L2 | reviewer | 1 | 653 | PASS |  |  |  |
| P7.L3 | reviewer | 1 | 571 | FAIL |  |  |  |

## Merges

- P1.R1: rounds=1 outcome=pass
- P1.R3: rounds=1 outcome=pass
- P1.R2: rounds=1 outcome=pass
- P1.R4: rounds=2 outcome=pass
- P2.C1: rounds=3 outcome=pass
- P3.A1: rounds=1 outcome=pass
- P3.A2: rounds=1 outcome=pass
- P3.A3: rounds=1 outcome=pass
- P3.A4: rounds=1 outcome=pass
- P4.P1: rounds=2 outcome=pass
- P5.WP01: rounds=2 outcome=pass
- P5.WP02: rounds=1 outcome=pass
- P5.WP03: rounds=1 outcome=pass
- P5.WP07: rounds=2 outcome=pass
- P5.WP06: rounds=1 outcome=pass
- P6.H: rounds=1 outcome=pass
- P6.F2: rounds=1 outcome=pass
- P6.F1: rounds=1 outcome=pass
- P6.F3: rounds=1 outcome=pass
- P6.F4: rounds=1 outcome=pass
