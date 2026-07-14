/*
 * Project-owned test harness for the pinned external perfect6502 oracle.
 *
 * This file is compiled beside the exact, separately acquired perfect6502.c
 * path after every upstream input has been hash-checked. No upstream source or
 * generated binary belongs in the repository.
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <types.h>
#include <netlist_sim.h>

extern void *initAndResetChip(void);
extern void destroyChip(void *state);
extern void step(void *state);
extern uint16_t readPC(void *state);
extern uint8_t readSP(void *state);
extern uint8_t readP(void *state);
extern BOOL readRW(void *state);
extern uint16_t readAddressBus(void *state);
extern uint8_t readDataBus(void *state);
extern uint8_t readIR(void *state);
extern uint8_t memory[65536];

enum {
    PANDA_SYNC_NODE = 539,
    PANDA_IRQ_NODE = 103,
    PANDA_NMI_NODE = 1297,
    PANDA_RESET_NODE = 159
};

static void drive_line(
    void *state,
    const char *scenario,
    unsigned long cycle_index,
    unsigned long assert_cycle);

static void full_cycle(
    void *state,
    const char *scenario,
    unsigned long index,
    unsigned long assert_cycle)
{
    step(state);
    /* Apply pin transitions while clk0 is low, before the serviced phi2-high half. */
    drive_line(state, scenario, index, assert_cycle);
    step(state);
    printf(
        "%lu SYNC=%u %c %04X %02X PC=%04X SP=%02X P=%02X IR=%02X\n",
        index,
        isNodeHigh(state, PANDA_SYNC_NODE),
        readRW(state) ? 'R' : 'W',
        readAddressBus(state),
        readDataBus(state),
        readPC(state),
        readSP(state),
        readP(state),
        readIR(state));
}

static void fill_program(const char *scenario)
{
    memset(memory, 0xea, sizeof(memory));
    memory[0xfffc] = 0x00;
    memory[0xfffd] = 0x80;
    memory[0xfffe] = 0x00;
    memory[0xffff] = 0x90;
    memory[0xfffa] = 0x00;
    memory[0xfffb] = 0xa0;
    memory[0x8000] = 0xa2; /* LDX #$FD */
    memory[0x8001] = 0xfd;
    memory[0x8002] = 0x9a; /* TXS */
    memory[0x8003] = 0x58; /* CLI */
    memory[0x8004] = 0xea; /* NOP */
    memory[0x8005] = 0xea; /* NOP */
    memory[0x8006] = 0x00; /* BRK */
    memory[0x8007] = 0xea; /* BRK padding */
    if (strcmp(scenario, "branch-not") == 0) {
        memory[0x8004] = 0xf0; /* BEQ not taken because Z is clear */
        memory[0x8005] = 0x00;
        memory[0x8006] = 0xea;
    } else if (strcmp(scenario, "branch-taken") == 0) {
        memory[0x8004] = 0xd0; /* BNE taken, same page */
        memory[0x8005] = 0x00;
        memory[0x8006] = 0xea;
    } else if (strcmp(scenario, "branch-cross") == 0) {
        memory[0x8004] = 0xd0; /* BNE taken to $7FFF */
        memory[0x8005] = 0xf9;
        memory[0x7fff] = 0xea;
    }
    memory[0x9000] = 0xea;
    memory[0xa000] = 0xea;
}

static int parse_cycle(const char *text, unsigned long *value)
{
    char *end = NULL;
    *value = strtoul(text, &end, 10);
    return end != text && *end == '\0' && *value <= 1000;
}

static void drive_line(
    void *state,
    const char *scenario,
    unsigned long cycle_index,
    unsigned long assert_cycle)
{
    if (cycle_index == assert_cycle) {
        if (strcmp(scenario, "irq") == 0 || strncmp(scenario, "branch-", 7) == 0) {
            setNode(state, PANDA_IRQ_NODE, 0);
        } else if (strcmp(scenario, "nmi") == 0 || strcmp(scenario, "brk-nmi") == 0) {
            setNode(state, PANDA_NMI_NODE, 0);
        } else if (strcmp(scenario, "reset") == 0) {
            setNode(state, PANDA_RESET_NODE, 0);
        } else if (strcmp(scenario, "irq-nmi") == 0) {
            setNode(state, PANDA_NMI_NODE, 0);
        }
    }
    if ((strcmp(scenario, "nmi") == 0 || strcmp(scenario, "brk-nmi") == 0) &&
        cycle_index == assert_cycle + 2) {
        setNode(state, PANDA_NMI_NODE, 1);
    }
    if (strcmp(scenario, "reset") == 0 && cycle_index == assert_cycle + 3) {
        setNode(state, PANDA_RESET_NODE, 1);
    }
}

int main(int argc, char **argv)
{
    const char *scenario = "none";
    unsigned long assert_cycle = 1001;
    unsigned long cycles = 36;
    if (argc == 4) {
        scenario = argv[1];
        if (strcmp(scenario, "irq") != 0 && strcmp(scenario, "nmi") != 0 &&
            strcmp(scenario, "reset") != 0 && strcmp(scenario, "none") != 0) {
            if (strcmp(scenario, "irq-nmi") != 0 && strcmp(scenario, "branch-not") != 0 &&
                strcmp(scenario, "branch-taken") != 0 && strcmp(scenario, "branch-cross") != 0) {
                if (strcmp(scenario, "brk-nmi") != 0) {
                    fputs("unknown oracle scenario\n", stderr);
                    return 2;
                }
            }
        }
        if (!parse_cycle(argv[2], &assert_cycle) || !parse_cycle(argv[3], &cycles)) {
            fputs("cycle arguments must be integers from 0 through 1000\n", stderr);
            return 2;
        }
        if (strcmp(scenario, "irq-nmi") == 0 && assert_cycle < 17) {
            fputs("irq-nmi NMI assertion must occur after the bootstrap poll\n", stderr);
            return 2;
        }
    } else if (argc != 1) {
        fputs(
            "usage: perfect6502-oracle "
            "[none|irq|nmi|reset|irq-nmi|brk-nmi|branch-not|branch-taken|branch-cross "
            "ASSERT_CYCLE TOTAL_CYCLES]\n",
            stderr);
        return 2;
    }

    fill_program(scenario);
    void *state = initAndResetChip();
    if (state == NULL) {
        fputs("perfect6502 initialization failed\n", stderr);
        return 1;
    }
    if (strcmp(scenario, "irq-nmi") == 0) {
        setNode(state, PANDA_IRQ_NODE, 0);
    }
    for (unsigned long i = 1; i <= cycles; i++) {
        full_cycle(state, scenario, i, assert_cycle);
    }
    destroyChip(state);
    return 0;
}
