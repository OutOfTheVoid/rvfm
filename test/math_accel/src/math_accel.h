#ifndef MATH_ACCEL_H
#define MATH_ACCEL_H

#define MA_BASE 0xF0070000
#define MA_REG(n) *((volatile uint32_t *)(MA_BASE | (n << 2)))
#define MA_LOAD_V2(n) *((volatile uint32_t *)(MA_BASE | ((n + 64) << 2)))
#define MA_STORE_V2(n) *((volatile uint32_t *)(MA_BASE | ((n + 80) << 2)))
#define MA_LOAD_V3(n) *((volatile uint32_t *)(MA_BASE | ((n + 96) << 2)))
#define MA_STORE_V3(n) *((volatile uint32_t *)(MA_BASE | ((n + 112) << 2)))
#define MA_LOAD_V4(n) *((volatile uint32_t *)(MA_BASE | ((n + 128) << 2)))
#define MA_STORE_V4(n) *((volatile uint32_t *)(MA_BASE | ((n + 144) << 2)))
#define MA_ERROR *((volatile uint32_t *)(MA_BASE | ((n + 254) << 2)))
#define MA_CMD *((volatile uint32_t *)(MA_BASE | (255 << 2)))

#define MA_CMD_V_V_OP_V(v_a, v_b, op, v_dest) (op | (v_a << 8) | (v_b << 12) | (v_dest << 16))
#define MA_CMD_V_V_OP_R(v_a, v_b, op, r_dest) (op | (v_a << 8) | (v_b << 12) | (r_dest << 16))
#define MA_CMD_V_OP_R(v, op, r_dest) (op | (v << 8) | (r_dest << 12))
#define MA_CMD_V_R_OP_V(v, r, op, v_dest) (op | (v << 8) | (r << 12) | (v_dest << 18))
#define MA_CMD_R_R_OP_R(r_a, r_b, op, r_dest) (op | (r_a << 8) | (r_b << 14) | (r_dest << 20))
#define MA_CMD_R_OP_R(r, op, r_dest) (op | (r << 8) | (r_dest << 14))
#define MA_CMD_V_OP_V(v, op, v_dest) (op | (v << 8) | (v_dest << 12))

#define MA_OP_ADD2 0x00
#define MA_OP_ADD3 0x01
#define MA_OP_ADD4 0x02

#define MA_OP_SUB2 0x03
#define MA_OP_SUB3 0x04
#define MA_OP_SUB4 0x05

#define MA_OP_MUL2 0x06
#define MA_OP_MUL3 0x07
#define MA_OP_MUL4 0x08

#define MA_OP_DIV2 0x09
#define MA_OP_DIV3 0x0A
#define MA_OP_DIV4 0x0B

#define MA_OP_REM2 0x0C
#define MA_OP_REM3 0x0D
#define MA_OP_REM4 0x0E

#define MA_OP_POW2 0x0F
#define MA_OP_POW3 0x10
#define MA_OP_POW4 0x11

#define MA_OP_PROJECT2 0x12
#define MA_OP_PROJECT3 0x13
#define MA_OP_PROJECT4 0x14

#define MA_OP_CROSS 0x15

#define MA_OP_QROTATE 0x16
#define MA_OP_QMUL 0x17

#define MA_OP_DOT2 0x20
#define MA_OP_DOT3 0x21
#define MA_OP_DOT4 0x22

#define MA_OP_LENGTH2 0x40
#define MA_OP_LENGTH3 0x41
#define MA_OP_LENGTH4 0x42

#define MA_OP_NORM2 0x50
#define MA_OP_NORM3 0x51
#define MA_OP_NORM4 0x52

#define MA_OP_SCALE2 0x60
#define MA_OP_SCALE3 0x61
#define MA_OP_SCALE4 0x62
#define MA_OP_ANGLEAXISQUAT 0x63
#define MA_OP_ROTATE 0x64

#define MA_OP_R_ADD 0x80
#define MA_OP_R_SUB 0x81
#define MA_OP_R_MUL 0x82
#define MA_OP_R_DIV 0x83
#define MA_OP_R_REM 0x84
#define MA_OP_R_POW 0x85
#define MA_OP_R_ATAN2 0x86
#define MA_OP_R_LOG 0x87

#define MA_OP_SIN 0xA0
#define MA_OP_COS 0xA1
#define MA_OP_TAN 0xA2
#define MA_OP_ARCSIN 0xA3
#define MA_OP_ARCCOS 0xA4
#define MA_OP_ARCTAN 0xA5
#define MA_OP_EXP 0xA6
#define MA_OP_LN 0xA7
#define MA_OP_INV 0xA8

#define MA_OP_QSLERP 0xC0

#endif
