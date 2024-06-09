#include <stdint.h>

typedef struct {
    uint8_t type;           // 0x0
    uint8_t trigger_key;    // 0x1
    uint8_t gap2[2];        // 0x2
    uint8_t material;       // 0x4
    uint8_t transparent;    // 0x5
    uint8_t gap6[2];        // 0x6
    int16_t x;              // 0x8
    int16_t y;              // 0xA
    uint16_t color;         // 0xC
    uint16_t bg_color;      // 0xE
    uint8_t gap10[4];       // 0x10
    uint32_t text[11];      // 0x14
} ui_layer_t;

extern uint32_t g_key_single_count[3];
extern uint8_t g_key_analog[3];
extern uint8_t g_button_state[3];

void draw_rect(uint16_t* fb, int32_t x, int32_t y, uint32_t width, uint32_t height, uint16_t color);
void draw_number(uint16_t* framebuffer, int32_t x, int32_t y, uint32_t num, uint32_t max_nums, uint16_t color, uint16_t bg_color, uint32_t font_size, uint8_t transparent);

void ms_callback_orig();

const uint32_t o3cpatch_str[] = U"o3cpatch!";

__attribute__((naked)) void handle_reset_custom() {
    __asm__(
        // Pasted from startup_ch32v30x_D8.S
        /* Load data section from flash to RAM */
        "la     a0, _data_lma;"
        "la     a1, _data_vma;"
        "la     a2, _edata;"
        "bgeu   a1, a2, 2f;"
    "1:"
        "lw     t0, (a0);"
        "sw     t0, (a1);"
        "addi   a0, a0, 4;"
        "addi   a1, a1, 4;"
        "bltu   a1, a2, 1b;"
    "2:"
        /* Clear bss section */
        "la     a0, _sbss;"
        "la     a1, _ebss;"
        "bgeu   a0, a1, 2f;"
    "1:"
        "sw     zero, (a0);"
        "addi   a0, a0, 4;"
        "bltu   a0, a1, 1b;"
    "2:"
        "j      handle_reset_orig;"
    );
}

__attribute__((naked)) void get_analog_key() {
    __asm__(
        // a0 (x10) = input
        // a1 (x11) = output
        // a4 (x14) = scratch
        // a5 (x15) = command id/scratch
        "li      a4, 0x69;"
        "bne     a5, a4, handle_usb_cmd_2_fail;"

        // Write input to key colors
        "la      a5, g_key_color_state;"
        "lbu     a4, 0x3(a0);"
        "sb      a4, 0xB(a5);"
        "lbu     a4, 0x4(a0);"
        "sb      a4, 0xD(a5);"
        "lbu     a4, 0x5(a0);"
        "sb      a4, 0xF(a5);"
        "lbu     a4, 0x6(a0);"
        "sb      a4, 0x1B(a5);"
        "lbu     a4, 0x7(a0);"
        "sb      a4, 0x1D(a5);"
        "lbu     a4, 0x8(a0);"
        "sb      a4, 0x1F(a5);"

        // Write analog values to output
        "la      a5, g_key_analog;"
        "c.li    a4, 0x10;"
        "sb      a4, 0x1(a1);"
        "sb      zero, 0x2(a1);"
        "lw      a4, 0x0(a5);"
        "sw      a4, 0x4(a1);"

        "j       handle_usb_cmd_2_ret;"
    );
}

void draw_horizontal_line(uint16_t* fb, uint32_t x, uint32_t y, uint32_t width, uint16_t color) {
    if (x >= 160 || y >= 80)
        return;
    if (x + width >= 160)
        width = 160 - x;
    for (uint32_t i = x; i < x + width; i++)
        fb[i + 160 * y] = color;
}

void draw_vertical_line(uint16_t* fb, uint32_t x, uint32_t y, uint32_t height, uint16_t color) {
    if (x >= 160 || y >= 80)
        return;
    if (y + height >= 80)
        height = 80 - y;
    for (uint32_t i = y; i < y + height; i++)
        fb[x + 160 * i] = color;
}

// Already exists but this is better
void key_pressure_horizontal(uint16_t* fb, ui_layer_t* layer) {
    uint8_t raw = g_key_analog[layer->trigger_key];
    draw_rect(fb, layer->x, layer->y + 4, raw - (raw >> 3), 8, layer->color);

    if (g_button_state[layer->trigger_key]) {
        draw_rect(fb, layer->x + 80, layer->y, 16, 16, layer->color);
    } else {
        draw_horizontal_line(fb, layer->x + 80, layer->y, 16, layer->color);
        draw_horizontal_line(fb, layer->x + 80, layer->y + 15, 16, layer->color);
        draw_vertical_line(fb, layer->x + 80, layer->y, 16, layer->color);
        draw_vertical_line(fb, layer->x + 80 + 15, layer->y, 16, layer->color);
    }
}

static uint32_t g_kps_last_key_count[3];
static uint8_t g_kps_buf[1000];
static uint8_t g_kps_buf_idx;
static uint8_t g_kps_sub;
void keys_per_second(uint16_t* fb, ui_layer_t* layer) {
    uint32_t sum = 0;
    for (int i = 0; i < 1000; i++)
        sum += g_kps_buf[i];
    draw_number(fb, layer->x, layer->y, sum, 2, layer->color, layer->bg_color, 16, layer->transparent);
}

void custom_widget_handler(uint16_t* fb, ui_layer_t* layer) {
    switch (layer->material) {
        case 16:
            key_pressure_horizontal(fb, layer);
            break;
        case 17:
            keys_per_second(fb, layer);
            break;
    }
}

void ms_callback_custom() {
    if ((g_kps_sub++ & 3) == 0) {
        g_kps_buf[g_kps_buf_idx] = 0;
        for (int i = 0; i < 3; i++) {
            if (g_kps_last_key_count[i] != g_key_single_count[i]) {
                g_kps_buf[g_kps_buf_idx]++;
                g_kps_last_key_count[i] = g_key_single_count[i];
            }
        }
        g_kps_buf_idx = (g_kps_buf_idx + 1) % 1000;
    }

    ms_callback_orig();
}

__attribute__((naked)) void screen_layer_update_custom() {
    __asm__(
        // s1 (x9) = framebuffer ptr
        // a4 (x14) = max widget idx/scratch
        // &sp[0x30] = layer
        "bgeu   a4, a5, (screen_layer_update_hook + 4);"

        "mv     a0, s1;"
        "addi   a1, sp, 0x30;"
        "jal    ra, custom_widget_handler;"

        "j      screen_layer_update_ret;"
    );
}

__attribute__((naked)) void screen_layer_update_menu_custom() {
    __asm__(
        // a1 (x11) = layer idx
        // a4 (x14) = scratch
        "beq    a1, zero, screen_layer_update_no_menu;"

        "li     a4, 1;"
        "bne    a1, a4, screen_layer_update_ret;"

        "jal    ra, menu_tick;"
        "j      screen_layer_update_ret;"
    );
}
