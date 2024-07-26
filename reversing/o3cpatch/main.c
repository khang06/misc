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

typedef struct {
    uint32_t* text;
    void (*func)();
} menu_item_t;

extern uint32_t g_key_single_count[3];
extern uint8_t g_key_analog[3];
extern uint8_t g_key_pressed[3];

extern menu_item_t g_menu_items[];
extern uint8_t g_menu_item_count;

void draw_number(uint16_t* framebuffer, int32_t x, int32_t y, uint32_t num, uint32_t max_nums, uint16_t color, uint16_t bg_color, uint32_t font_size, uint8_t transparent);
void draw_ascii_char(uint16_t* framebuffer, int32_t x, int32_t y, uint16_t char_idx, uint16_t color, uint16_t bg_color, uint32_t font_size, uint8_t transparent);

void ms_callback_orig();
void menu_device();

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

static void draw_horizontal_line(uint16_t* fb, uint32_t x, uint32_t y, uint32_t width, uint16_t color) {
    if (x >= 160 || y >= 80)
        return;
    if (x + width >= 160)
        width = 160 - x;
    for (uint32_t i = x; i < x + width; i++)
        fb[i + 160 * y] = color;
}

static void draw_vertical_line(uint16_t* fb, uint32_t x, uint32_t y, uint32_t height, uint16_t color) {
    if (x >= 160 || y >= 80)
        return;
    if (y + height >= 80)
        height = 80 - y;
    for (uint32_t i = y; i < y + height; i++)
        fb[x + 160 * i] = color;
}

// Already exists in the firmware but this is better
static void draw_rect(uint16_t* fb, int32_t x, int32_t y, uint32_t width, uint32_t height, uint16_t color) {
    if (x >= 160 || y >= 80)
        return;
    if (y + height >= 80)
        height = 80 - y;
    for (uint32_t i = y; i < y + height; i++)
        draw_horizontal_line(fb, x, i, width, color);
}

// Already exists but this is customized a bit
static void key_pressure_horizontal(uint16_t* fb, ui_layer_t* layer) {
    uint8_t raw = g_key_analog[layer->trigger_key];
    draw_rect(fb, layer->x, layer->y + 4, raw - (raw >> 3), 8, layer->color);

    if (g_key_pressed[layer->trigger_key]) {
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
static void keys_per_second(uint16_t* fb, ui_layer_t* layer) {
    uint32_t sum = 0;
    for (int i = 0; i < 1000; i++)
        sum += g_kps_buf[i];
    draw_number(fb, layer->x, layer->y, sum, 2, layer->color, layer->bg_color, 16, layer->transparent);
}

// Period is 65536 instead of 2pi
// Returns a number from 0 to 65534
static const uint16_t g_not_sin_lookup[] = {
	0x0000, 0x0195, 0x032A, 0x04BF, 0x0654, 0x07E9, 0x097D, 0x0B11,
	0x0CA4, 0x0E37, 0x0FCA, 0x115C, 0x12ED, 0x147D, 0x160D, 0x179C,
	0x192A, 0x1AB7, 0x1C42, 0x1DCD, 0x1F57, 0x20DF, 0x2266, 0x23EC,
	0x2570, 0x26F3, 0x2874, 0x29F4, 0x2B72, 0x2CEE, 0x2E69, 0x2FE2,
	0x3158, 0x32CD, 0x3440, 0x35B1, 0x3720, 0x388D, 0x39F7, 0x3B5F,
	0x3CC5, 0x3E29, 0x3F8A, 0x40E8, 0x4244, 0x439E, 0x44F5, 0x4649,
	0x479A, 0x48E9, 0x4A34, 0x4B7D, 0x4CC3, 0x4E06, 0x4F45, 0x5082,
	0x51BB, 0x52F2, 0x5425, 0x5554, 0x5681, 0x57AA, 0x58CF, 0x59F2,
	0x5B10, 0x5C2B, 0x5D43, 0x5E56, 0x5F66, 0x6073, 0x617B, 0x6280,
	0x6381, 0x647E, 0x6577, 0x666C, 0x675D, 0x684A, 0x6933, 0x6A18,
	0x6AF8, 0x6BD5, 0x6CAD, 0x6D81, 0x6E50, 0x6F1C, 0x6FE3, 0x70A5,
	0x7164, 0x721D, 0x72D3, 0x7384, 0x7430, 0x74D8, 0x757B, 0x7619,
	0x76B3, 0x7749, 0x77D9, 0x7865, 0x78EC, 0x796F, 0x79EC, 0x7A65,
	0x7ADA, 0x7B49, 0x7BB3, 0x7C19, 0x7C7A, 0x7CD6, 0x7D2D, 0x7D7F,
	0x7DCC, 0x7E14, 0x7E58, 0x7E96, 0x7ED0, 0x7F04, 0x7F34, 0x7F5E,
	0x7F84, 0x7FA4, 0x7FC0, 0x7FD6, 0x7FE8, 0x7FF4, 0x7FFC, 0x7FFF,
	0x7FFF
};
static uint16_t not_sin(uint16_t x) {
	uint16_t idx = (x >> 7) & 0x7F;
	uint16_t frac = x & 0x7F;
	if (x & 0x4000) {
		// flip x
		idx = 0x7F - idx;
		frac = 0x7F - frac;
	}
	uint16_t interp = ((uint32_t)g_not_sin_lookup[idx] * (0x80 - frac) + g_not_sin_lookup[idx + 1] * frac) >> 7;
	return (x & 0x8000) ? (0x7FFF - interp) : (0x7FFF + interp); // flip y
}

// g_menu_ms doesn't seem to be that accurate sometimes
static uint32_t g_uptime_ms;

static void plasma(uint16_t* fb, ui_layer_t* layer) {
    uint32_t t = g_uptime_ms << 2;
    for (int y = 0; y < 80; y++) {
		for (int x = 0; x < 160; x++) {
			uint32_t s1 = not_sin(x * 1234 + t);
			uint32_t s2 = not_sin(y * 1432 - (t << 1));
			uint32_t s3 = not_sin((((x - y) * 5678 - (t >> 2)) >> 3));
			uint32_t s4 = not_sin((((x + y) * 2768 + (t >> 3)) >> 2));

			uint8_t sum = (s1 + s2 + s3 + s4) >> 10;
			fb[x + 160 * y] = ((sum >> 3) << 11) | (sum >> 2) | (sum >> 3);
		}
	}
}

// Max uptime is ~1193 hours (49.7 days) before overflowing
static void uptime(uint16_t* fb, ui_layer_t* layer) {
    static char str_base[] = "0000:00:00";
    char* str_ptr = &str_base[6];
    int32_t offset = 56;

    // TODO: Probably not the most efficient way to do this
    uint32_t hours = g_uptime_ms / 3600000;
    uint32_t minutes = (g_uptime_ms / 60000) % 60;
    uint32_t seconds = (g_uptime_ms / 1000) % 60;

    str_base[9] = seconds % 10 + '0';
    str_base[8] = seconds / 10 + '0';
    str_base[6] = minutes % 10 + '0';
    if (hours != 0 || minutes >= 10) {
        str_base[5] = minutes / 10 + '0';
        str_ptr--;
        offset -= 8;
    }
    if (hours != 0) {
        str_ptr--;
        while (hours != 0) {
            *(--str_ptr) = hours % 10 + '0';
            offset -= 8;
            hours /= 10;
        }
    }

    for (; *str_ptr != '\0'; str_ptr++) {
        draw_ascii_char(fb, layer->x + offset, layer->y, *str_ptr, layer->color, layer->bg_color, 16, layer->transparent);
        offset += 8;
    }
}

void custom_widget_handler(uint16_t* fb, ui_layer_t* layer) {
    switch (layer->material) {
        case 16:
            key_pressure_horizontal(fb, layer);
            break;
        case 17:
            keys_per_second(fb, layer);
            break;
        case 18:
            plasma(fb, layer);
            break;
        case 19:
            uptime(fb, layer);
            break;
    }
}

void ms_callback_custom() {
    // TODO: I think I messed this up because I shouldn't need g_kps_sub here
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
    g_uptime_ms++;

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

void menu_device_info_custom() {
    g_menu_items[0].text = U"Back >>";
    g_menu_items[1].text = U"o3cpatch|khangaroo";
    g_menu_items[2].text = U"built "__DATE__;
    g_menu_items[3].text = U"fw: v1.5 20240709";

    for (int i = 0; i < 4; i++)
        g_menu_items[i].func = menu_device;

    g_menu_item_count = 4;
}
