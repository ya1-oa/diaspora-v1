#ifndef DIASPORA_WEAVE_RESEARCH_H
#define DIASPORA_WEAVE_RESEARCH_H
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Research limits. No production consensus or finality guarantee. */
#define W_MAX_WIDTH 8
#define W_MAX_CYCLES 192
#define W_MAX_ARCHIVES 3
#define W_ID_SIZE 32
#define W_PAYLOAD_SIZE 48

typedef struct {
    uint32_t wave, slot, width, bits;
    uint8_t parents[W_MAX_WIDTH][W_ID_SIZE];
    uint8_t archives[W_MAX_ARCHIVES][W_ID_SIZE];
    uint32_t archive_count;
    char payload[W_PAYLOAD_SIZE];
    uint64_t strand_nonce[W_MAX_WIDTH];
    uint64_t binder_nonce;
    uint8_t id[W_ID_SIZE];
} w_cycle;

typedef struct {
    uint32_t width, bits, complete_waves, pending_count, count;
    uint8_t genesis[W_MAX_WIDTH][W_ID_SIZE];
    w_cycle cycles[W_MAX_CYCLES];
    w_cycle pending[W_MAX_WIDTH];
} w_lattice;

typedef enum {
    W_OK=0, W_BAD_ARGUMENT=-1, W_BAD_WAVE=-2, W_BAD_PARENTS=-3,
    W_BAD_ARCHIVES=-4, W_BAD_PROOF=-5, W_BAD_ID=-6,
    W_DUPLICATE_SLOT=-7, W_CAPACITY=-8
} w_status;

w_status w_init(w_lattice *lattice, uint32_t width, uint32_t difficulty_bits);
w_status w_mine(const w_lattice *lattice, uint32_t slot, const char *payload, w_cycle *out);
w_status w_validate(const w_lattice *lattice, const w_cycle *cycle);
w_status w_submit(w_lattice *lattice, const w_cycle *cycle);
w_status w_replay(const w_lattice *lattice);
size_t w_descendants(const w_lattice *lattice, const uint8_t source[W_ID_SIZE]);
uint64_t w_expected_hashes(const w_lattice *lattice);
void w_hex(const uint8_t id[W_ID_SIZE], char out[65]);
const char *w_status_string(w_status status);
/* Structural digest is SHA3-256 with domain-separated, fixed-size encoding. */
#endif
