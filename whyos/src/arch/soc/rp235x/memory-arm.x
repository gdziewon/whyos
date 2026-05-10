/* # Used by cortex-m-rt crate,
 * https://docs.rs/cortex-m-rt/0.7.5/cortex_m_rt/index.html#memoryx
 *
 * values based on RP-008373-DS-2-rp2350-datasheet.pdf */

MEMORY {
    /* External flash (XIP) */
    FLASH : ORIGIN = 0x10000000, LENGTH = 4096K

    /* SRAM0-3 and SRAM4-7 are always striped on bits 3:2 of the address */
    RAM : ORIGIN = 0x20000000, LENGTH = 512K

    /* non-striped SRAM8-9 banks */
    SRAM8 : ORIGIN = 0x20080000, LENGTH = 4K
    SRAM9 : ORIGIN = 0x20081000, LENGTH = 4K
}

SECTIONS {
    /* # Boot ROM info (Image Definition)
     *
     * Goes after .vector_table
     */
    .start_block : ALIGN(4)
    {
        __start_block_addr = .; /* global variable with addr of start block */
        KEEP(*(.start_block)); /* linker should keep these blocks, even if unused */
        KEEP(*(.boot_info));
    } > FLASH

} INSERT AFTER .vector_table;

/* update .text section to start after the boot info */
_stext = ADDR(.start_block) + SIZEOF(.start_block); /* link.x variable */

SECTIONS {
    /* # Metadata */
    .bi_entries : ALIGN(4)
    {
        __bi_entries_start = .;
        KEEP(*(.bi_entries));
        . = ALIGN(4);
        __bi_entries_end = .;
    } > FLASH
} INSERT AFTER .text;

SECTIONS {
    /* # Signature for Secure Boot */
    .end_block : ALIGN(4)
    {
        __end_block_addr = .;
        KEEP(*(.end_block));
    } > FLASH

} INSERT AFTER .uninit;

/* Inform Boot ROM about image size */
PROVIDE(start_to_end = __end_block_addr - __start_block_addr);
PROVIDE(end_to_start = __start_block_addr - __end_block_addr); /* reverse pointer */


