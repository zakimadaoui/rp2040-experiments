# Bus behavior characterization
### Introduction

In this set of experiments we will try to get Core0 and Core1 of the rp2040 to execute concurrent memory accesses in lockstep and try to determine how the AHB-Lite Crossbar arbiters resolve the concurrent accesses of both cores to the same Memory bank (same bus slave) and try to set an upper bound to the latency resulting from the arbitration.

### What we already know from the rp2040 datasheet

**Execution time of some assembly instructions**

| Instruction             | Clock Cycles |
| ----------------------- | ------------ |
| `nop`                   | 1            |
| `LDR Rd, [Rn, #<imm>] ` | 2            |
| `STR Rd, [Rn, #<imm>]`  | 2            |

**Table1:** execution time of NOP, STR and LDR instruction

**Note** that the cycle counts are based on a system with zero wait-states.

**From Section 2.1.1.1. Bus Priority:** 

- If multiple masters of the same priority level attempt to access the same slave simultaneously, a round-robin tie break is applied, i.e. the arbiter grants access to each master in turn.

- Priority arbitration only applies to multiple masters attempting to access the same slave on the same cycle. Accesses to different slaves, e.g. different SRAM banks, can proceed simultaneously.
- When accessing a slave with zero wait states, such as SRAM (i.e. can be accessed once per system clock cycle), high-priority masters will never observe any slowdown or other timing effects caused by accesses from low-priority masters. This allows guaranteed latency and throughput for hard real time use cases; it does however mean a low-priority master may get stalled until there is a free cycle.

The above bullet points were taken directly from the rp2040 datasheet. From them we can understand that when both cores have the same bus priority and they **simultaneously** execute a load or store instruction to the same memory bank, the arbiter will apply a round robin tie break. And if we assume the round robin tie break goes in this order (core0 then core1) then the execution should go as follows for `LDR R0, [R1] ` (where on both cores R0 points to an address in the same memory bank):

| Time                 | Core0 executing LDR                                          | Core1 executing LDR                                          |
| -------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| t=0                  | put the address (stored in R1) to the address bus            | put the address (stored in R1) to the address bus            |
| t=1                  | arbiter grants access to **core0**. so data is copied from memory to destination register | arbiter puts core1 in **wait** state                         |
| t=2                  | .... (some other instruction starts executing)               | arbiter grants access to **core1**. so data is copied from memory to destination register |
| total cycles for LDR | **2 clock cycles**                                           | **3 clock cycles**                                           |

**Table2**: Expected behavior of LDR instruction in race condition between masters with same bus priority 

Later on we will verify whether this is true or not !

### Preparing for experiments

#### Measuring clock cycles accurately

In order to prepare for the next experiments, we need accurately measure the clock cycles taken to execute one or more instructions. But because a Data Watchpoint trigger is not implemented for the cortex-m0/m0+ (more specifically the CYCCNT register in DWT), we need to use the Systick timer for the same purpose.

In [ARM developers documentation](https://developer.arm.com/documentation/ka001406/latest/) it is mentioned that: 

 the processor can use its SysTick function to measure elapsed cycle counts. The SysTick function must be configured to use the processor clock as the reference timing source.

```c
// Systick regs
int *STCSR = (int *)0xE000E010;                    
int *STRVR = (int *)0xE000E014;              
int *STCVR = (int *)0xE000E018;
// Configure Systick    
*STRVR = 0xFFFFFF;  // max count
*STCVR = 0;         // force a re-load of the counter value register
*STCSR = 5;         // enable FCLK count without interrupt
```
The cycle count for an operation can then be obtained by reading the STCVR immediately before and immediately after the operation in question. Because STCVR is a down counter, the number of core clock cycles taken by the operation is given by:

``` c
(STCVR1 - STCVR2 - 2)
```

The overhead of two cycles is because the read of the STCVR is Strongly-Ordered with regard to other memory accesses or data processing instructions.



The above technique is used in `examples/pre_exp1.rs` example to measure the clock cycles taken by 100 read, write and NOP instructions on both cores.

**Assumption/Expected result**

based on the cycle count needed for LDR, STR and NOP operations mentioned in the table above:

- 100 reads take 200 clock cycles
- 100 writes take 200 clock cycles
- 100 NOPs take 100 clock cycles

**Obtained Results**

```bash
cd bus_behavior
cargo run --example pre_exp1
```

Output:

```
Running at 125 MHz
100 reads  on core 0 => 200 clock cycles
100 writes on core 0 => 200 clock cycles
100 NOPs   on core 0 => 100 clock cycles
100 reads  on core 1 => 200 clock cycles
100 writes on core 1 => 200 clock cycles
100 NOPs   on core 1 => 100 clock cycles
```

The results match with the expected clock cycles count, and therefore we can conclude that Systick timer can be used to accurately measure clock cycles taken for certain operations

#### Executing an instruction on both cores at the same time

Theoretically, we can get the two cores to execute in lockstep by tying to execute an interrupt on both cores at the same time. It would be even better to be able to trigger different interrupt lines on each core at the same time, and that could be done on the rp2040 as follows:

- unmask TIMER0 interrupt on core0, and wait for core1 to unmask TIMER1 interrupt from its local NVIC
- unmask TIMER1 interrupt on core1 and let core 0 this has been done
- core0 then Sets the bits for Alarm0 and Alarm1 interrupts to 1 in the Interrupt **enable** register INTE of the TIMER peripheral. 
- core0 then Sets the bits for Alarm0 and Alarm1 interrupts to 1 in the Interrupt **Force** register INTF of the TIMER peripheral. This will trigger an interrupt on both cores, core0 will handle the IRQ for Alarm0 (TIMER_IRQ_0) and core1 will handle the IRQ for Alarm1 (TIMER_IRQ_1).



**Note:** Before continuing to read the next paragraphs, I would suggest reading all the next experiments sections first (or at least comeback and read again the next paragraphs after you have completed reading this whole article) as they provide a lot of background that is needed to understand the next paragraphs. 



In practice the above steps (bullet points) are not enough to get the two cores to execute in lockstep. The reason is that when the interrupt lines are asserted on both cores, they both try to access the vector table to get the ISR address for that specific interrupt line. And this is the first place where we will have concurrent access of both cores to the same memory bank and the bus arbiter will grant access to one of the cores first, then, gives access to the other core (this will be explained in detail in the later experiments). So, Assuming that the vector table was stored in RAM the first core can fetch the ISR, then the second core will be able to fetch the other ISR one clock cycle after. And because of that the two cores will be out of sync.

To fix the earlier issue we can simply add a NOP instruction on one of the ISRs (the ISR which starts executing first) in order to synchronize the execution between the two cores and get them to execute in lockstep. However, if the vector table is stored in FLASH The wait time is significantly longer. I have measured It will be 140 clock cycles to be exact.

the linker script that comes with the RP2040 HAL puts the vector table in FLASH and the default bootloader does't offer copying the vector table to RAM so this has to be manually done. In order to avoid this hassle i will be adding 140 NOP instructions in one of the ISRs in all of the experiments in order to synchronize the two Cores to execute in lockstep. In the future i might modify the linkerscript and manually copy the vector table to RAM. Till then, don't be surprised to see 140 NOP instructions in the ISRs in examples codes.

----

### Experiments

In the next experiments we try to get the two cores (Core0 and Core1) to execute some memory access instructions in lockstep and play on the following parameters to understand how the bust arbitration works. The variable parameters are:

- Bus priority
- Data location
- Code location

Systick timer will be used to measure how long the simulanious accesses take, in addition, rp2040 has some **bus performance counters** that can count interesting events such as the events of completion of access to the APB arbiter which was previously delayed due to an access by another master.

----

#### Experiment 1 (examples/exp1.rs)

Execute a memory read instruction to the same memory location/bank on **both cores at the same time**

- both cores have the same priority (priorities 0 and 1 gave the same results)
- core0 executes TIMER0 ISR which is located in **SRAM2**
- core1 executes TIMER1 ISR which is located in **SRAM3**
- shared data in **SRAM4** (at **0x20040000**) is accessed from both ISRs at the same time
- Bus performance counters, from BUSCTRL peripheral, are used to count events of **contested APB accesses** on sram4  
- Systick timer on each core is used to measure clock cycles of concurrent read instructions



**Expectations**

The expected result of this experiment is discussed in **Table2** above, where on one core, executing the LDR instruction takes 3 clock cycles, while it takes 2 clock cycles on the other core.

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp1
```

```
concurrent read on core 0 took 3 clock cycles. read val is 77
concurrent read on core 1 took 2 clock cycles. read val is 77
contested RAM accesses [sram4 = 1]
```

The result came out similarly to what was expected, core1 executed the LDR instruction in 2 clock cycles while core0 took 3 clock cycles. The bus performace counter further confirms that there was one APB master that had to be delayed **once** due to access by another master. 

| Time                 | Core1 executing LDR                                          | Core0 executing LDR                                          |
| -------------------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| t=0                  | put the address (stored in R1) to the address bus            | put the address (stored in R1) to the address bus            |
| t=1                  | arbiter grants access to **core0**. so data is copied from memory to destination register | arbiter puts core1 in **wait** state                         |
| t=2                  |                                                              | arbiter grants access to **core1**. so data is copied from memory to destination register |
| total cycles for LDR | **2 clock cycles**                                           | **3 clock cycles**                                           |

----

### Experiment 1.1 (examples/exp1_1.rs)

Same as experiment 1, with the following differences:

- each core tries to access local data (core0 accesses data in sram2, and core1 accesses data in sram3)

**Expectations**

No concurrent access, both cores execute the LDR instruction in 2 clock cycles

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp1_1
```

```
concurrent read on core 0 took 2 clock cycles. read val is 2936059264
concurrent read on core 1 took 2 clock cycles. read val is 2936059264
contested RAM accesses [sram4 = 0]
```

The results were the same as expected

----

### Experiment 2 (examples/exp2.rs)

Same as experiment 1, with the following differences:

- core1 has bus priority 1, core0 has bus priority 0

**Expectations**

same as experiment1

**Obtained results**

Also the same as experiment 1

----

### Experiment 3 (examples/exp3.rs)

Same as experiment 1, with the following differences:

- core0 has bus priority 1, core1 has bus priority 0

**Expectations**

core0 executes the LDR instruction in 2 clock cycles while core0 takes 3 clock cycles to execute.

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp3
```

```
concurrent read on core 0 took 2 clock cycles. read val is 77
concurrent read on core 1 took 3 clock cycles. read val is 77
contested RAM accesses [sram4 = 1]
```

The results meet the expectation

----

### Experiment 4 (examples/exp4.rs)

Same as experiment 1, with the following differences:

- core 0 makes a read operation, core1 make a write operation

**Expectations**

No changes to the results from experiment 1

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp4
```

```
shared sram4 data value before concurrent access is 77, core1 will write it to be 7 later
concurrent read on core 0 took 3 clock cycles. read val is 7
concurrent read on core 1 took 2 clock cycles. written val is 7
contested RAM accesses [sram4 = 1]
```

As expected, it is indifferent to the bus arbiter whether its a read or write operation, the arbiter will always perform a round robin tie break for the concurrent access, starting by core1 in the case where the bus priority is the same for both masters.

One thing that is worth noting in this experiment, it that we can always predict the outcome of a race condition when two cores simultaneously write (or one writes and the other reads) the same data. In this example the shared data had a value of 77, when the concurrent access happens core1 will always be granted access first, so it overwrites the value to 7 and then core0 is granted access to read the data which it sees the value 7 ! 

----

### Experiment 5 (examples/exp5.rs)

Same as experiment 1, with the following differences:

- 100 read instructions instead of one read instruction

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp2
```

```
concurrent read on core 0 took 201 clock cycles. read val is 77
concurrent read on core 1 took 200 clock cycles. read val is 77
contested RAM accesses [sram4 = 1]
```

**Explanation**

At the first glance one would expect that we would have at least 100 contested SRAM4 accesses as the read instructions will be executed in lock step. However if we examine closely, we will find that the obtained results are reasoble and hold true for an infinite amount of reads:

When the two cores start executing the read instructions in lock step, this will happen as soon as they try to execute the first concurrent read operation to the same memory bank:

| Time | Core1 executing LDR                                          | Core0 executing LDR                                          |
| ---- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| t=0  | put the address (stored in R1) to the address bus            | put the address (stored in R1) to the address bus            |
| t=1  | arbiter grants access to **core0**. so data is copied from memory to destination register | arbiter puts core1 in **wait** state                         |
| t=2  | starts executing the second LDR instruction. so  i put sthe address (stored in R1) to the address bus | arbiter grants access to **core1**. so data is copied from memory to destination register |
| t=3  | arbiter grants access to **core0**. so data is copied from memory to destination register | starts executing the second LDR instruction. so  i put sthe address (stored in R1) to the address bus |
| t=4  | ...                                                          | arbiter grants access to **core1**. so data is copied from memory to destination register |

Notice that at **t=2** Core1 has already finished executing the first LDR instruction and it starts executing the second one. This breaks the synchronization between the two cores and core1 will always becore 1 clock cycle ahead from core0. Which means there will be no more concurrent accesses to memory but instead on each clock cycle a different master requests acceess to memory which the arbiter will happily grants !  

----

### Experiment 6 (examples/exp6.rs)

Same as experiment 1, with the following differences:

- 100 read instructions instead of one read instruction
- core1 has higher bus priority than core0

**Expectations**

Same results as experiment 5

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp2
```

```
concurrent read on core 0 took 201 clock cycles. read val is 77
concurrent read on core 1 took 200 clock cycles. read val is 77
contested RAM accesses [sram4 = 1]
```

The results were the same as expected

----

### Experiment 7 (examples/exp7.rs + examples/exp7_1.rs)

Same as experiment 1, with the following differences:

- TIMER0 and TIMER1 ISR in same memory location (SRAM bank2). where core0 executes TIMER0 isr and core1 executes TIMER1 ISR, but the two cores try to access **the same** memory bank (sram4)

**Obtained results**

```bash
cd bus_behavior
cargo run --example exp7
```

```
concurrent read on core 0 took 2 clock cycles. read val is 77
concurrent read on core 1 took 2 clock cycles. read val is 77
contested RAM accesses [sram4 = 0]
```

**Explanation**

its hard to predict what would happen, But a logical explanation can be as follows.

The cortex-m0+ has a two stage instruction pipeline, when both cores try to concurrently fetch an instruction from the same RAM bank, one of them is granted access while the other **stalls**  (analogous to what happened in experiment 5) so the **instruction fetch** stages on the two cores will not be in sync and therefore there will be no contested accesses to SRAM4 as the two cores will not be executing instructions in lockstep but there's always one core that is one clock cycle ahead from the other core. And from previous experiments we know that core1 will be the core that will be granted access first and hence it will be the one that is 1 clock cycle ahead of core0 

In fact we can test whether this hypothesis is correct by adding a NOP instruction in TIMER1 ISR (to be executed by core1). The NOP instruction takes 1 clock cycle to execute and this should be enough to put back core1 one clock cycle behind and hence it will be back in sync with core0. This has been tried in `examples/exp7_1.rs`, and the output is as hyphothesised, we get a concurrent access !

```
concurrent read on core 0 took 3 clock cycles. read val is 77
concurrent read on core 1 took 2 clock cycles. read val is 77
contested RAM accesses [sram4 = 1]
```

----

### Conclusion

Through the previous experiment we can arrive to a conclusion about the timing behavior of the worst case scenario where multiple masters with the same priority all try to access the same memory bank at the same time.

First of all, in order to have bounded latency the master has to have the highest priority, since a lower priority master will wait forever for a higher priority master to finish. However, masters with the same highest priority will always have a bounded worst case latency on each shared concurrent memory access as the bus arbiter will always apply a round-robin tie break where it grants access to one of the contesting masters on each clock cycles until the concurrent access is resolved. 

In the case the rp2040, we have 4 bus masters: 

- core0
- core1
- DMA read
- DMA write. 

So, if we assume that all of those bus master have the same bus priority and they all attempt to access the same memory bank all at the same time, then if a round-robin tie break is applied by the arbiter, the last master request will be resolved after it wait for 3 clock cycles extra. So, if this was core0 for example trying to execute a LDR instruction, it will take 5 clock cycles at the very worse to finish executing this instruction (instead of 2 clock cycles) 

