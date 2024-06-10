custom_flash_base = 0x2A000;
custom_ram_base = 0x20013500;

handle_reset_orig = 0x4F60;
sayodevice_url_load = 0x78C2;

handle_usb_cmd_2_hook = 0xA0BE;
handle_usb_cmd_2_fail = 0x9CE0;
handle_usb_cmd_2_ret = 0x9CB8;

g_key_single_count = 0x20000490;
g_key_analog = 0x2000EA40;
g_key_color_state = 0x20012D4C;
g_key_pressed = 0x2000DA38;

g_menu_secs = 0x20000410;
g_menu_ms = 0x20000414;

screen_layer_update_menu_hook = 0x66FC;
screen_layer_update_no_menu = 0x671A;
screen_layer_update_hook = 0x6A9C;
screen_layer_update_ret = 0x6700;

menu_tick = 0x8D8A;
menu_tick_fb_clear = 0x8D96;

ms_callback_hook = 0xD6E0;
ms_callback_orig = 0x2979E;

draw_number = 0x5DCA;
draw_ascii_char = 0x59B0;

MEMORY {
	FLASH : ORIGIN = custom_flash_base, LENGTH = 0x3F00
	RAM : ORIGIN = custom_ram_base, LENGTH = 0x4000
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
