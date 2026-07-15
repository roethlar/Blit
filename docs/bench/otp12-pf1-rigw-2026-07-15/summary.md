# otp-12 pf-1 rig-W phase report

Validation: PASS — exact four-block OFF–ON–ON–OFF schedule, forward/reverse cell and role ordering, 8 valid role pairs per trace state/cell, trace-off and gRPC trace absence, and correlated two-role TCP terminal traces.

## Durable total wall-time summaries

| cell | trace | source total median ms | destination total median ms | Δ total ms | paired total d median ms | N_pair_split total ms | role-order drift total ms | paired range total ms | N_pair total ms |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| wm_tcp_mixed | off | 1469.5 | 1368.5 | -101 | -108 | 161 | 29 | 329 | 329 |
| wm_tcp_mixed | on | 1494.5 | 1367 | -127.5 | -133.5 | 47 | 26 | 100 | 100 |
| mw_tcp_mixed | off | 2253.5 | 2305.5 | 52 | 21.5 | 192 | 31.5 | 538 | 538 |
| mw_tcp_mixed | on | 2182.5 | 2311 | 128.5 | 167.5 | 62.5 | 93.5 | 682 | 682 |
| wm_grpc_mixed | off | 1660 | 1425 | -235 | -223.5 | 80 | 110 | 154 | 154 |
| wm_grpc_mixed | on | 1668.5 | 1443.5 | -225 | -225 | 59.5 | 14.5 | 101 | 101 |
| wm_tcp_large | off | 1569.5 | 1428 | -141.5 | -142 | 42.5 | 5 | 97 | 97 |
| wm_tcp_large | on | 1573 | 1430.5 | -142.5 | -143 | 9.5 | 7.5 | 23 | 23 |

The authoritative wall-time measurand is `total_ms = transfer_ms + (settled_ms - 250) + flush_ms`: client execution plus every millisecond beyond the common 250 ms observation budget and the destination durability probe. Only the common first 250 ms is excluded from summaries, deltas, distributions, observer bias, and resolution floors.

`Δ = median(destination_init total_ms) − median(source_init total_ms)`. Each paired `d_i = destination_init total_ms_i − source_init total_ms_i`. `N_pair_split = max(|median(d_1..d_4) − median(d_5..d_8)|, |median(d_odd) − median(d_even)|)`. The conservative operative The independent role-order drift is `|median(d_source-first) − median(d_destination-first)|`; the S,D,D,S schedule means this is not the odd/even partition. The conservative operative `N_pair = max(N_pair_split, role-order drift, max(d) − min(d))`, so a balanced bimodal mixture cannot produce a zero floor.

For target `wm_tcp_mixed`: Δ_off=-101 ms, Δ_on=-127.5 ms, observer_bias=|Δ_on−Δ_off|=26.5 ms, N_pair_off=329 ms, N_pair_on=100 ms, and N_resolution=329 ms.

This run measures the observer and paired resolution floors; it does not grade any hypothesis recovery.

## Sorted distributions and descriptive largest-gap modes

The split is descriptive only; it does not assert statistical modality.

| cell | trace | metric | sorted ms | largest gap ms | descriptive modes |
|---|---:|---|---|---:|---|
| wm_tcp_mixed | off | source_init total_ms | 1342;1448;1451;1461;1478;1505;1551;1553 | 106 | [1342] | [1448;1451;1461;1478;1505;1551;1553] |
| wm_tcp_mixed | off | destination_init total_ms | 1324;1339;1349;1360;1377;1386;1389;1578 | 189 | [1324;1339;1349;1360;1377;1386;1389] | [1578] |
| wm_tcp_mixed | off | paired total_ms d | -202;-193;-166;-124;-92;-72;35;127 | 107 | [-202;-193;-166;-124;-92;-72] | [35;127] |
| wm_tcp_mixed | on | source_init total_ms | 1446;1453;1479;1491;1498;1520;1549;1550 | 29 | [1446;1453;1479;1491;1498;1520] | [1549;1550] |
| wm_tcp_mixed | on | destination_init total_ms | 1324;1340;1341;1365;1369;1384;1389;1403 | 24 | [1324;1340;1341] | [1365;1369;1384;1389;1403] |
| wm_tcp_mixed | on | paired total_ms d | -181;-160;-158;-138;-129;-117;-107;-81 | 26 | [-181;-160;-158;-138;-129;-117;-107] | [-81] |
| mw_tcp_mixed | off | source_init total_ms | 2108;2111;2118;2245;2262;2299;2330;2373 | 127 | [2108;2111;2118] | [2245;2262;2299;2330;2373] |
| mw_tcp_mixed | off | destination_init total_ms | 1821;2203;2254;2291;2320;2327;2338;2349 | 382 | [1821] | [2203;2254;2291;2320;2327;2338;2349] |
| mw_tcp_mixed | off | paired total_ms d | -297;-119;-96;-3;46;76;209;241 | 178 | [-297] | [-119;-96;-3;46;76;209;241] |
| mw_tcp_mixed | on | source_init total_ms | 2113;2116;2125;2152;2213;2236;2278;2294 | 61 | [2113;2116;2125;2152] | [2213;2236;2278;2294] |
| mw_tcp_mixed | on | destination_init total_ms | 1860;2277;2289;2299;2323;2331;2335;2461 | 417 | [1860] | [2277;2289;2299;2323;2331;2335;2461] |
| mw_tcp_mixed | on | paired total_ms d | -434;45;53;161;174;183;218;248 | 479 | [-434] | [45;53;161;174;183;218;248] |
| wm_grpc_mixed | off | source_init total_ms | 1569;1577;1611;1648;1672;1683;1692;1696 | 37 | [1569;1577;1611] | [1648;1672;1683;1692;1696] |
| wm_grpc_mixed | off | destination_init total_ms | 1395;1418;1423;1425;1425;1425;1448;1464 | 23 | [1395] | [1418;1423;1425;1425;1425;1448;1464] |
| wm_grpc_mixed | off | paired total_ms d | -301;-267;-258;-247;-200;-154;-151;-147 | 47 | [-301;-267;-258;-247] | [-200;-154;-151;-147] |
| wm_grpc_mixed | on | source_init total_ms | 1629;1654;1663;1665;1672;1675;1683;1683 | 25 | [1629] | [1654;1663;1665;1672;1675;1683;1683] |
| wm_grpc_mixed | on | destination_init total_ms | 1401;1405;1423;1439;1448;1451;1453;1469 | 18 | [1401;1405] | [1423;1439;1448;1451;1453;1469] |
| wm_grpc_mixed | on | paired total_ms d | -282;-278;-249;-236;-214;-201;-194;-181 | 29 | [-282;-278] | [-249;-236;-214;-201;-194;-181] |
| wm_tcp_large | off | source_init total_ms | 1486;1507;1565;1569;1570;1575;1576;1576 | 58 | [1486;1507] | [1565;1569;1570;1575;1576;1576] |
| wm_tcp_large | off | destination_init total_ms | 1409;1421;1423;1427;1429;1429;1432;1442 | 12 | [1409] | [1421;1423;1427;1429;1429;1432;1442] |
| wm_tcp_large | off | paired total_ms d | -156;-153;-149;-147;-137;-133;-78;-59 | 55 | [-156;-153;-149;-147;-137;-133] | [-78;-59] |
| wm_tcp_large | on | source_init total_ms | 1569;1570;1571;1572;1574;1580;1580;1582 | 6 | [1569;1570;1571;1572;1574] | [1580;1580;1582] |
| wm_tcp_large | on | destination_init total_ms | 1416;1427;1428;1430;1431;1433;1440;1441 | 11 | [1416] | [1427;1428;1430;1431;1433;1440;1441] |
| wm_tcp_large | on | paired total_ms d | -155;-154;-147;-143;-143;-139;-139;-132 | 7 | [-155;-154] | [-147;-143;-143;-139;-139;-132] |

## Phase evidence

`phase_events.csv` contains 11392 structured events. `phase_intervals.csv` contains 14964 local-clock intervals.

Each phase-event row carries the arm's validated `transfer_ms`, `settled_ms`, `flush_ms`, and authoritative `total_ms`.

Every interval uses `elapsed_ns` from one endpoint only. `unix_ns` is retained in the event export for provenance and is never used for cross-host subtraction.

## Clock-offset evidence

`clock_summary.csv` selects the minimum-RTT before and after sample for each of 128 scheduled arms and reports its midpoint offset. These samples document cross-host uncertainty only; no cross-host phase duration is computed or graded.
