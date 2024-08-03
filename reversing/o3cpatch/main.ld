target_fw = 20240803;
custom_flash_base = 0x2AE20;
custom_ram_base = 0x200134E8;

handle_reset_orig = 0x5048;
curbrk_init = 0x2AD88;

handle_usb_cmd_2_hook = 0xA41E;
handle_usb_cmd_2_fail = 0x9F68;
handle_usb_cmd_2_ret = 0x9F40;

g_key_single_count = 0x200004E4;
g_key_analog = 0x2000E9B8;
g_key_color_state = 0x20012DB8;
g_key_pressed = 0x2000D9B0;

g_menu_items = 0x20012BE0;
g_menu_item_count = 0x20012DA0;

screen_layer_update_menu_hook = 0x6798;
screen_layer_update_no_menu = 0x67B6;
screen_layer_update_hook = 0x6C3C;
screen_layer_update_ret = 0x679C;
screen_layer_update_boot_screen = 0x6866;

menu_tick = 0x8FB8;
menu_tick_fb_clear = 0x8FC4;

menu_device = 0x7B16;
menu_device_info = 0x79AA;

ms_callback_hook = 0xDBC2;
ms_callback_orig = 0x2A96E;

draw_number = 0x5EB2;
draw_ascii_char = 0x5A98;

g_font = 0x19AD8;

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
