#pragma once

#include <stdint.h>

namespace Rand {
    extern uint32_t State;

    inline void Seed(uint32_t seed) {
        State = seed;
    }
    inline uint32_t Next() {
        // xorshift32
        uint32_t x = State;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        State = x;
        return x;
    }
    inline int32_t Range(int32_t start, int32_t end) { // inclusive
        return Next() % (end - start + 1) + start; // Yes, there's modulo bias. I don't care
    }
    inline float NextFloat() {
        union {
            uint32_t i;
            float f;
        } u;
        u.i = 0x3F800000 | (Next() >> 9); // [1, 2)
        return u.f - 1.0f; // [0, 1)
    }
    inline float RangeFloat(float start, float end) { // inclusive start, exclusive end
        return NextFloat() * (end - start) + start;
    }
}

__declspec(noreturn) void Panic(const char* msg, ...);