#define __IO
#define __I
#define __O

typedef __I uint64_t vuc64;  /* Read Only */
typedef __I uint32_t vuc32;  /* Read Only */
typedef __I uint16_t vuc16;  /* Read Only */
typedef __I uint8_t vuc8;   /* Read Only */

typedef const uint64_t uc64;  /* Read Only */
typedef const uint32_t uc32;  /* Read Only */
typedef const uint16_t uc16;  /* Read Only */
typedef const uint8_t uc8;   /* Read Only */

typedef __I int64_t vsc64;  /* Read Only */
typedef __I int32_t vsc32;  /* Read Only */
typedef __I int16_t vsc16;  /* Read Only */
typedef __I int8_t vsc8;   /* Read Only */

typedef const int64_t sc64;  /* Read Only */
typedef const int32_t sc32;  /* Read Only */
typedef const int16_t sc16;  /* Read Only */
typedef const int8_t sc8;   /* Read Only */

typedef __IO uint64_t  vu64;
typedef __IO uint32_t  vu32;
typedef __IO uint16_t vu16;
typedef __IO uint8_t  vu8;

typedef uint64_t  u64;
typedef uint32_t  u32;
typedef uint16_t u16;
typedef uint8_t  u8;

typedef __IO int64_t  vs64;
typedef __IO int32_t  vs32;
typedef __IO int16_t  vs16;
typedef __IO int8_t   vs8;

typedef int64_t  s64;
typedef int32_t  s32;
typedef int16_t s16;
typedef int8_t  s8;

typedef struct
{
    __IO u32 CTLR;
    __IO u32 SR;
    __IO u64 CNT;
    __IO u64 CMP;
}SysTick_Type;

typedef struct
{
  __IO uint16_t CTLR1;
  uint16_t  RESERVED0;
  __IO uint16_t CTLR2;
  uint16_t  RESERVED1;
  __IO uint16_t SMCFGR;
  uint16_t  RESERVED2;
  __IO uint16_t DMAINTENR;
  uint16_t  RESERVED3;
  __IO uint16_t INTFR;
  uint16_t  RESERVED4;
  __IO uint16_t SWEVGR;
  uint16_t  RESERVED5;
  __IO uint16_t CHCTLR1;
  uint16_t  RESERVED6;
  __IO uint16_t CHCTLR2;
  uint16_t  RESERVED7;
  __IO uint16_t CCER;
  uint16_t  RESERVED8;
  __IO uint16_t CNT;
  uint16_t  RESERVED9;
  __IO uint16_t PSC;
  uint16_t  RESERVED10;
  __IO uint16_t ATRLR;
  uint16_t  RESERVED11;
  __IO uint16_t RPTCR;
  uint16_t  RESERVED12;
  __IO uint16_t CH1CVR;
  uint16_t  RESERVED13;
  __IO uint16_t CH2CVR;
  uint16_t  RESERVED14;
  __IO uint16_t CH3CVR;
  uint16_t  RESERVED15;
  __IO uint16_t CH4CVR;
  uint16_t  RESERVED16;
  __IO uint16_t BDTR;
  uint16_t  RESERVED17;
  __IO uint16_t DMACFGR;
  uint16_t  RESERVED18;
  __IO uint16_t DMAADR;
  uint16_t  RESERVED19;
} TIM_TypeDef;
