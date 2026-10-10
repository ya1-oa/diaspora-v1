#include "weave.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int count=0;
#define CHECK(expression) do { if(!(expression)){fprintf(stderr,"FAIL %s:%d: %s\n",__FILE__,__LINE__,#expression);exit(1);} }while(0)
#define TEST(name) do {name();printf("PASS %-38s\n",#name);++count;}while(0)

static void mine_wave(w_lattice *l) {
    w_cycle mine[W_MAX_WIDTH];
    for(uint32_t s=0;s<l->width;s++){
        char payload[48];snprintf(payload,sizeof(payload),"wave%u:slot%u",l->complete_waves+1,s);
        CHECK(w_mine(l,s,payload,&mine[s])==W_OK);
    }
    /* Submit out of order: non-permanent slot identity and pending waves. */
    for(uint32_t s=l->width;s>0;s--)CHECK(w_submit(l,&mine[s-1])==W_OK);
}
static void rejects_configuration(void){
    w_lattice l;
    CHECK(w_init(NULL,3,8)==W_BAD_ARGUMENT);
    CHECK(w_init(&l,2,8)==W_BAD_ARGUMENT);
    CHECK(w_init(&l,9,8)==W_BAD_ARGUMENT);
    CHECK(w_init(&l,3,0)==W_BAD_ARGUMENT);
    CHECK(w_init(&l,3,19)==W_BAD_ARGUMENT);
    CHECK(w_init(&l,3,8)==W_OK);
}
static void genesis_deterministic(void){
    w_lattice a,b;CHECK(w_init(&a,3,8)==W_OK);CHECK(w_init(&b,3,8)==W_OK);
    CHECK(memcmp(a.genesis,b.genesis,sizeof(a.genesis))==0);
    CHECK(a.count==0&&a.complete_waves==0);
}
static void mines_and_replays(void){
    for(uint32_t width=3;width<=8;width+=2){
        w_lattice l;CHECK(w_init(&l,width,6)==W_OK);
        for(int i=0;i<3;i++)mine_wave(&l);
        CHECK(l.count==3*width);CHECK(l.complete_waves==3);
        CHECK(w_replay(&l)==W_OK);
        CHECK(w_expected_hashes(&l)==3ULL*width*(width+1)*64);
    }
}
static void wrong_parents_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.parents[0][0]^=1;
    CHECK(w_validate(&l,&c)==W_BAD_PARENTS);
}
static void wrong_archives_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.archives[0][0]^=1;
    CHECK(w_validate(&l,&c)==W_BAD_ARCHIVES);
}
static void tampered_payload_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.payload[0]='z';
    CHECK(w_validate(&l,&c)!=W_OK);
}
static void tampered_proof_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.strand_nonce[0]++;
    CHECK(w_validate(&l,&c)!=W_OK);
}
static void tampered_binder_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,11)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.binder_nonce++;
    CHECK(w_validate(&l,&c)!=W_OK);
}
static void tampered_id_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.id[0]^=1;
    CHECK(w_validate(&l,&c)==W_BAD_ID);
}
static void duplicate_slot_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    CHECK(w_submit(&l,&c)==W_OK);
    CHECK(w_submit(&l,&c)==W_DUPLICATE_SLOT);
    CHECK(w_replay(&l)==W_OK);
}
static void future_wave_rejected(void){
    w_lattice l;w_cycle c;CHECK(w_init(&l,3,8)==W_OK);
    CHECK(w_mine(&l,1,"a",&c)==W_OK);
    c.wave=2;
    CHECK(w_validate(&l,&c)==W_BAD_WAVE);
}
static void descendant_cone(void){
    w_lattice l;CHECK(w_init(&l,3,8)==W_OK);
    for(int i=0;i<4;i++)mine_wave(&l);
    CHECK(w_descendants(&l,l.cycles[0].id)==9);
    CHECK(w_descendants(&l,l.cycles[9].id)==0);
    CHECK(w_replay(&l)==W_OK);
}
static void fork_worlds_are_distinct(void){
    w_lattice honest,adversary;
    CHECK(w_init(&honest,3,7)==W_OK);
    mine_wave(&honest);
    adversary=honest;
    w_cycle h,a;
    CHECK(w_mine(&honest,0,"honest-ledger",&h)==W_OK);
    CHECK(w_mine(&adversary,0,"attacker-ledger",&a)==W_OK);
    CHECK(memcmp(h.id,a.id,32)!=0);
    CHECK(w_submit(&honest,&h)==W_OK);
    CHECK(w_submit(&adversary,&a)==W_OK);
    CHECK(w_replay(&honest)==W_OK);
    CHECK(w_replay(&adversary)==W_OK);
}
static void archival_refs_replay_through_epochs(void){
    w_lattice l;CHECK(w_init(&l,5,4)==W_OK);
    for(int i=0;i<5;i++)mine_wave(&l);
    CHECK(w_replay(&l)==W_OK);
    CHECK(l.cycles[20].archive_count==3);
}
static void reject_capacity(void){
    w_lattice l;CHECK(w_init(&l,8,1)==W_OK);
    for(int i=0;i<24;i++)mine_wave(&l);
    CHECK(l.count==W_MAX_CYCLES);
    w_cycle c;CHECK(w_mine(&l,0,"capacity",&c)==W_OK);
    CHECK(w_submit(&l,&c)==W_CAPACITY);
}
int main(void){
    TEST(rejects_configuration);
    TEST(genesis_deterministic);
    TEST(mines_and_replays);
    TEST(wrong_parents_rejected);
    TEST(wrong_archives_rejected);
    TEST(tampered_payload_rejected);
    TEST(tampered_proof_rejected);
    TEST(tampered_binder_rejected);
    TEST(tampered_id_rejected);
    TEST(duplicate_slot_rejected);
    TEST(future_wave_rejected);
    TEST(descendant_cone);
    TEST(fork_worlds_are_distinct);
    TEST(archival_refs_replay_through_epochs);
    TEST(reject_capacity);
    printf("ALL %d TESTS PASSED\n",count);
    return 0;
}
