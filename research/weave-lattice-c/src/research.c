#include "weave.h"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <inttypes.h>

/* Baseline gambler's ruin, not a complete network/lattice security model. */
static uint64_t state=42;
static uint64_t rng(void){state^=state>>12;state^=state<<25;state^=state>>27;return state*UINT64_C(2685821657736338717);}
static double sample01(void){return (double)(rng()>>11)/9007199254740992.0;}
static void attack_benchmark(int trials,int max_rounds,uint64_t seed){
    const double q[]={.30,.45,.51,.55,.70,.99};
    state=seed?seed:1;
    printf("MODEL: PoW deficit random walk. Defender starts 6 accepted units ahead.\n");
    printf("MODEL: each round gives 1 unit to the attacker with probability q, otherwise defender.\n");
    printf("MODEL: no lattice scheduling, consensus, network delays, witnesses, or finality.\n");
    printf("hash_share,successes,trials,success_fraction,round_limit\n");
    for(size_t i=0;i<sizeof(q)/sizeof(q[0]);i++){
        int wins=0;
        for(int run=0;run<trials;run++){
            int deficit=6;
            for(int step=0;step<max_rounds;step++){
                deficit+=sample01()<q[i]?-1:1;
                if(deficit<=0){wins++;break;}
            }
        }
        printf("%.2f,%d,%d,%.6f,%d\n",q[i],wins,trials,(double)wins/trials,max_rounds);
    }
}
int main(int argc,char **argv){
    int width=3,waves=4,bits=8,attack=0,trials=20000,rounds=100;
    uint64_t seed=42;
    for(int i=1;i<argc;i++){
        if(strcmp(argv[i],"--attack")==0)attack=1;
        else if(i+1<argc && strcmp(argv[i],"--width")==0)width=atoi(argv[++i]);
        else if(i+1<argc && strcmp(argv[i],"--waves")==0)waves=atoi(argv[++i]);
        else if(i+1<argc && strcmp(argv[i],"--bits")==0)bits=atoi(argv[++i]);
        else if(i+1<argc && strcmp(argv[i],"--trials")==0)trials=atoi(argv[++i]);
        else if(i+1<argc && strcmp(argv[i],"--rounds")==0)rounds=atoi(argv[++i]);
        else if(i+1<argc && strcmp(argv[i],"--seed")==0)seed=(uint64_t)strtoull(argv[++i],NULL,10);
        else {fprintf(stderr,"Usage: weave-research [--width 3..8 --waves N --bits 1..18] or [--attack --trials N --rounds N --seed N]\n");return 2;}
    }
    if(attack){if(trials<=0||trials>10000000||rounds<=0||rounds>100000){fprintf(stderr,"bad simulation bounds\n");return 2;}attack_benchmark(trials,rounds,seed);return 0;}
    if(width<3||width>8||waves<1||waves*width>W_MAX_CYCLES||bits<1||bits>18){fprintf(stderr,"invalid width/waves/bits\n");return 2;}
    w_lattice l;w_status status=w_init(&l,(uint32_t)width,(uint32_t)bits);
    if(status!=W_OK)return 1;
    for(int w=0;w<waves;w++){
        w_cycle pending[W_MAX_WIDTH];
        for(int s=0;s<width;s++){
            char payload[W_PAYLOAD_SIZE];snprintf(payload,sizeof(payload),"wave:%d:slot:%d",w+1,s);
            status=w_mine(&l,(uint32_t)s,payload,&pending[s]);
            if(status!=W_OK){fprintf(stderr,"mine: %s\n",w_status_string(status));return 1;}
        }
        for(int s=width-1;s>=0;s--){
            status=w_submit(&l,&pending[s]);
            if(status!=W_OK){fprintf(stderr,"submit: %s\n",w_status_string(status));return 1;}
        }
    }
    status=w_replay(&l);if(status!=W_OK){fprintf(stderr,"replay: %s\n",w_status_string(status));return 1;}
    char first_id[65];w_hex(l.cycles[0].id,first_id);
    printf("{\"model\":\"synchronous-coupled-cycle-DAG\",\"width\":%d,\"waves\":%d,\"cycles\":%u,\"verified\":true,\"difficulty_bits\":%d,\"expected_unique_sha3_trials\":%" PRIu64 ",\"rewritten_cycle\":\"%s\",\"invalidated_descendants\":%zu}\n",
           width,waves,l.count,bits,w_expected_hashes(&l),first_id,w_descendants(&l,l.cycles[0].id));
    return 0;
}
