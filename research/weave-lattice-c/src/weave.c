#include "weave.h"
#include <openssl/evp.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* All integers are canonical big-endian, every hash uses a separate domain. */
typedef struct { EVP_MD_CTX *ctx; bool ok; } hasher;
static hasher h_open(const char *domain) {
    hasher h={EVP_MD_CTX_new(),false};
    if (h.ctx && EVP_DigestInit_ex(h.ctx, EVP_sha3_256(), NULL)==1 &&
        EVP_DigestUpdate(h.ctx, domain, strlen(domain))==1) h.ok=true;
    return h;
}
static void h_bytes(hasher *h, const void *p, size_t n) {
    if (!h->ok || EVP_DigestUpdate(h->ctx,p,n)!=1) h->ok=false;
}
static void h_u32(hasher *h, uint32_t value) {
    uint8_t buf[4]; for(int i=3;i>=0;i--){buf[i]=(uint8_t)value;value>>=8;} h_bytes(h,buf,4);
}
static void h_u64(hasher *h, uint64_t value) {
    uint8_t buf[8]; for(int i=7;i>=0;i--){buf[i]=(uint8_t)value;value>>=8;} h_bytes(h,buf,8);
}
static bool h_close(hasher *h, uint8_t out[32]) {
    unsigned int length=0;
    bool ok=h->ok && EVP_DigestFinal_ex(h->ctx,out,&length)==1 && length==32;
    EVP_MD_CTX_free(h->ctx); h->ctx=NULL;return ok;
}
static bool equal_id(const uint8_t a[32],const uint8_t b[32]) {return memcmp(a,b,32)==0;}
static int compare_ids(const void *a,const void *b) {return memcmp(a,b,32);}
static bool hash_genesis(uint32_t width,uint32_t index,uint8_t id[32]) {
    hasher h=h_open("DIASPORA:L0:GENESIS");h_u32(&h,width);h_u32(&h,index);return h_close(&h,id);
}
static bool hash_base(const w_cycle *c, uint8_t out[32]) {
    hasher h=h_open("DIASPORA:L0:CYCLE-BASE");
    h_u32(&h,c->wave);h_u32(&h,c->slot);h_u32(&h,c->width);h_u32(&h,c->bits);
    for(uint32_t i=0;i<c->width;i++) h_bytes(&h,c->parents[i],32);
    h_u32(&h,c->archive_count);
    for(uint32_t i=0;i<c->archive_count;i++)h_bytes(&h,c->archives[i],32);
    /* fixed-width payload makes serialization unambiguous, also binds trailing bytes */
    h_bytes(&h,c->payload,sizeof(c->payload));return h_close(&h,out);
}
static bool hash_strand(const uint8_t base[32],uint32_t index,const uint8_t parent[32],uint64_t nonce,uint8_t out[32]) {
    hasher h=h_open("DIASPORA:L0:STRAND");h_bytes(&h,base,32);h_u32(&h,index);
    h_bytes(&h,parent,32);h_u64(&h,nonce);return h_close(&h,out);
}
static bool hash_binder(const uint8_t base[32],uint32_t width,uint8_t strands[W_MAX_WIDTH][32],uint64_t nonce,uint8_t out[32]) {
    hasher h=h_open("DIASPORA:L0:BINDER");h_bytes(&h,base,32);h_u32(&h,width);
    for(uint32_t i=0;i<width;i++)h_bytes(&h,strands[i],32);
    h_u64(&h,nonce);return h_close(&h,out);
}
static bool hash_cycle_id(const uint8_t base[32],const w_cycle *c,uint8_t out[32]) {
    hasher h=h_open("DIASPORA:L0:CYCLE-ID");h_bytes(&h,base,32);
    for(uint32_t i=0;i<c->width;i++)h_u64(&h,c->strand_nonce[i]);
    h_u64(&h,c->binder_nonce);return h_close(&h,out);
}
static bool meets_bits(const uint8_t hash[32],uint32_t bits) {
    uint32_t whole=bits/8,remaining=bits%8;
    for(uint32_t i=0;i<whole;i++)if(hash[i]!=0)return false;
    return !remaining || ((hash[whole] & (0xffu << (8-remaining)))==0);
}
static void current_parents(const w_lattice *l,uint8_t out[W_MAX_WIDTH][32]) {
    if(l->complete_waves==0)memcpy(out,l->genesis,l->width*32);
    else {
        uint32_t base=(l->complete_waves-1)*l->width;
        for(uint32_t i=0;i<l->width;i++)memcpy(out[i],l->cycles[base+i].id,32);
    }
    qsort(out,l->width,32,compare_ids);
}
static bool archives_for(const w_lattice *l,uint32_t wave,uint32_t slot,
                         uint8_t out[W_MAX_ARCHIVES][32],uint32_t *n) {
    uint8_t pool[W_MAX_CYCLES+W_MAX_WIDTH][32];uint32_t pool_n=0;
    for(uint32_t i=0;i<l->width;i++)memcpy(pool[pool_n++],l->genesis[i],32);
    if(wave>=3){
        uint32_t historical_count=(wave-2)*l->width;
        if(historical_count>l->count)return false;
        for(uint32_t i=0;i<historical_count;i++)memcpy(pool[pool_n++],l->cycles[i].id,32);
    }
    qsort(pool,pool_n,32,compare_ids);
    *n=pool_n<W_MAX_ARCHIVES?pool_n:W_MAX_ARCHIVES;
    bool selected[W_MAX_CYCLES+W_MAX_WIDTH]={false};
    for(uint32_t i=0;i<*n;i++) {
        hasher h=h_open("DIASPORA:L0:ARCHIVE-SELECTION");h_u32(&h,wave);h_u32(&h,slot);h_u32(&h,i);
        /* Seed depends only on canonical past state and wave/slot, not miner VRF choice. */
        for(uint32_t j=0;j<l->width;j++){
            uint8_t frontier[W_MAX_WIDTH][32];current_parents(l,frontier);
            h_bytes(&h,frontier[j],32);
        }
        uint8_t digest[32];if(!h_close(&h,digest))return false;
        uint64_t selector=0;for(int k=0;k<8;k++)selector=(selector<<8)|digest[k];
        uint32_t remaining=pool_n-i;
        uint32_t rank=(uint32_t)(selector % remaining);
        uint32_t choice=0;
        for(uint32_t j=0;j<pool_n;j++)if(!selected[j]){
            if(rank==0){choice=j;break;}rank--;
        }
        selected[choice]=true;memcpy(out[i],pool[choice],32);
    }
    return true;
}

w_status w_init(w_lattice *l,uint32_t width,uint32_t bits) {
    if(!l||width<3||width>W_MAX_WIDTH||bits<1||bits>18)return W_BAD_ARGUMENT;
    memset(l,0,sizeof(*l));l->width=width;l->bits=bits;
    for(uint32_t i=0;i<width;i++)if(!hash_genesis(width,i,l->genesis[i]))return W_BAD_ARGUMENT;
    return W_OK;
}
w_status w_validate(const w_lattice *l,const w_cycle *c) {
    if(!l||!c)return W_BAD_ARGUMENT;
    if(c->wave!=l->complete_waves+1||c->slot>=l->width||c->width!=l->width||c->bits!=l->bits)return W_BAD_WAVE;
    if(!memchr(c->payload,'\0',sizeof(c->payload)))return W_BAD_ARGUMENT;
    uint8_t parents[W_MAX_WIDTH][32]={0};current_parents(l,parents);
    if(memcmp(parents,c->parents,l->width*32)!=0)return W_BAD_PARENTS;
    uint8_t archives[W_MAX_ARCHIVES][32]={0};uint32_t archive_count=0;
    if(!archives_for(l,c->wave,c->slot,archives,&archive_count)||
       archive_count!=c->archive_count||memcmp(archives,c->archives,archive_count*32))return W_BAD_ARCHIVES;
    uint8_t base[32],strands[W_MAX_WIDTH][32]={0},digest[32];
    if(!hash_base(c,base))return W_BAD_PROOF;
    for(uint32_t i=0;i<c->width;i++){
        if(!hash_strand(base,i,c->parents[i],c->strand_nonce[i],strands[i])||!meets_bits(strands[i],c->bits))return W_BAD_PROOF;
    }
    if(!hash_binder(base,c->width,strands,c->binder_nonce,digest)||!meets_bits(digest,c->bits))return W_BAD_PROOF;
    if(!hash_cycle_id(base,c,digest))return W_BAD_ID;
    return equal_id(digest,c->id)?W_OK:W_BAD_ID;
}
w_status w_mine(const w_lattice *l,uint32_t slot,const char *payload,w_cycle *out) {
    if(!l||!payload||!out||slot>=l->width||strlen(payload)>=W_PAYLOAD_SIZE)return W_BAD_ARGUMENT;
    w_cycle c={0};c.wave=l->complete_waves+1;c.slot=slot;c.width=l->width;c.bits=l->bits;
    strcpy(c.payload,payload);current_parents(l,c.parents);
    if(!archives_for(l,c.wave,slot,c.archives,&c.archive_count))return W_BAD_ARGUMENT;
    uint8_t base[32],strands[W_MAX_WIDTH][32]={0},digest[32];
    if(!hash_base(&c,base))return W_BAD_ARGUMENT;
    for(uint32_t i=0;i<c.width;i++){
        uint64_t nonce=0;
        do {
            if(!hash_strand(base,i,c.parents[i],nonce,strands[i]))return W_BAD_ARGUMENT;
            if(meets_bits(strands[i],c.bits))break;
            if(nonce==UINT64_MAX)return W_BAD_PROOF;
        }while(++nonce);
        c.strand_nonce[i]=nonce;
    }
    uint64_t nonce=0;
    do {
        if(!hash_binder(base,c.width,strands,nonce,digest))return W_BAD_ARGUMENT;
        if(meets_bits(digest,c.bits))break;
        if(nonce==UINT64_MAX)return W_BAD_PROOF;
    }while(++nonce);
    c.binder_nonce=nonce;
    if(!hash_cycle_id(base,&c,c.id))return W_BAD_ARGUMENT;
    if(w_validate(l,&c)!=W_OK)return W_BAD_PROOF;
    *out=c;return W_OK;
}
w_status w_submit(w_lattice *l,const w_cycle *cycle) {
    if(!l||!cycle)return W_BAD_ARGUMENT;
    w_status valid=w_validate(l,cycle);if(valid!=W_OK)return valid;
    if(l->count+l->width>W_MAX_CYCLES)return W_CAPACITY;
    for(uint32_t i=0;i<l->pending_count;i++)if(l->pending[i].slot==cycle->slot)return W_DUPLICATE_SLOT;
    l->pending[l->pending_count++]=*cycle;
    if(l->pending_count==l->width){
        /* Stable slot order for deterministic state replay. */
        for(uint32_t slot=0;slot<l->width;slot++) {
            for(uint32_t j=0;j<l->width;j++)if(l->pending[j].slot==slot){l->cycles[l->count++]=l->pending[j];break;}
        }
        l->complete_waves++;l->pending_count=0;
    }
    return W_OK;
}
w_status w_replay(const w_lattice *l){
    if(!l)return W_BAD_ARGUMENT;
    w_lattice check;
    if(w_init(&check,l->width,l->bits)!=W_OK)return W_BAD_ARGUMENT;
    for(uint32_t i=0;i<l->count;i++){
        w_status result=w_submit(&check,&l->cycles[i]);if(result!=W_OK)return result;
    }
    for(uint32_t i=0;i<l->pending_count;i++){
        w_status result=w_submit(&check,&l->pending[i]);if(result!=W_OK)return result;
    }
    return check.count==l->count&&check.pending_count==l->pending_count ? W_OK:W_BAD_ARGUMENT;
}
static bool ref_contains(const w_cycle *c,const uint8_t id[32]){
    for(uint32_t i=0;i<c->width;i++)if(equal_id(c->parents[i],id))return true;
    for(uint32_t i=0;i<c->archive_count;i++)if(equal_id(c->archives[i],id))return true;
    return false;
}
size_t w_descendants(const w_lattice *l,const uint8_t source[32]) {
    if(!l||!source)return 0;
    uint8_t affected[W_MAX_CYCLES+1][32];size_t n=1;
    memcpy(affected[0],source,32);
    for(uint32_t i=0;i<l->count+l->pending_count;i++){
        const w_cycle *c=(i<l->count)?&l->cycles[i]:&l->pending[i-l->count];
        for(size_t j=0;j<n;j++)if(ref_contains(c,affected[j])){
            memcpy(affected[n++],c->id,32);break;
        }
    }
    return n-1;
}
uint64_t w_expected_hashes(const w_lattice *l){
    if(!l)return 0;
    return (uint64_t)(l->count+l->pending_count)*(l->width+1)*(1ULL<<l->bits);
}
void w_hex(const uint8_t id[32],char out[65]){
    static const char chars[]="0123456789abcdef";
    for(int i=0;i<32;i++){out[2*i]=chars[id[i]>>4];out[2*i+1]=chars[id[i]&15];}out[64]='\0';
}
const char *w_status_string(w_status status){
    switch(status){case W_OK:return "ok";case W_BAD_ARGUMENT:return "bad argument";
    case W_BAD_WAVE:return "wrong wave/parameters";case W_BAD_PARENTS:return "incorrect parents";
    case W_BAD_ARCHIVES:return "incorrect archives";case W_BAD_PROOF:return "invalid proof";
    case W_BAD_ID:return "invalid cycle id";case W_DUPLICATE_SLOT:return "duplicate slot";
    case W_CAPACITY:return "capacity exceeded";default:return "unknown";}
}
