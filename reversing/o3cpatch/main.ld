target_fw = 20241208;
custom_flash_base = 0x2C4D0;
custom_ram_base = 0x2001C000;

handle_reset_orig = 0x4F8C;
curbrk_init = 0x2C430;

handle_usb_cmd_2_hook = 0xA8EA;
handle_usb_cmd_2_fail = 0xA4BA;
handle_usb_cmd_2_ret = 0xA492;

g_key_single_count = 0x20010444;
g_key_idk = 0x200133C4;
g_key_color_state = 0x200132E4;
g_key_pressed = 0x2000DC30;

g_menu_items = 0x20013118;
g_menu_item_count = 0x2000CF64;

screen_layer_update_menu_hook = 0x66DC;
screen_layer_update_no_menu = 0x66FA;
screen_layer_update_hook = 0x6B80;
screen_layer_update_ret = 0x66E0;
screen_layer_update_boot_screen = 0x67AA;

menu_tick = 0x94DE;
menu_tick_fb_clear = 0x94F2;

menu_device = 0x7D4C;
menu_device_info = 0x7BE0;

ms_callback_hook = 0xEB3C;
ms_callback_orig = 0x2BD96;

draw_number = 0x5DF6;
draw_ascii_char = 0x59DC;

g_font = 0x1ADD8;

_end = _ebss;

MEMORY {
	FLASH : ORIGIN = custom_flash_base, LENGTH = 0x2DF80 - custom_flash_base
	RAM : ORIGIN = custom_ram_base, LENGTH = 0x2001F800 - custom_ram_base
}

SECTIONS {
	.text : {
		. = ALIGN(4);
		*(.text)
		*(.text.*)
		*(.rodata)
		*(.rodata*)
		*(.gnu.linkonce.t.*)
		. = ALIGN(4);
	} >FLASH AT>FLASH

    .dalign : {
		. = ALIGN(4);
		PROVIDE(_data_vma = .);
	} >RAM AT>FLASH	

	.dlalign : {
		. = ALIGN(4); 
		PROVIDE(_data_lma = .);
	} >FLASH AT>FLASH

    .data : {
    	*(.gnu.linkonce.r.*)
    	*(.data .data.*)
    	*(.gnu.linkonce.d.*)
		. = ALIGN(8);
    	PROVIDE( __global_pointer$ = . + 0x800 );
    	*(.sdata .sdata.*)
		*(.sdata2.*)
    	*(.gnu.linkonce.s.*)
    	. = ALIGN(8);
    	*(.srodata.cst16)
    	*(.srodata.cst8)
    	*(.srodata.cst4)
    	*(.srodata.cst2)
    	*(.srodata .srodata.*)
    	. = ALIGN(4);
		PROVIDE( _edata = .);
	} >RAM AT>FLASH

	.bss : {
		. = ALIGN(4);
		PROVIDE( _sbss = .);
  	    *(.sbss*)
        *(.gnu.linkonce.sb.*)
		*(.bss*)
     	*(.gnu.linkonce.b.*)		
		*(COMMON*)
		. = ALIGN(4);
		PROVIDE( _ebss = .);
	} >RAM AT>FLASH
}
