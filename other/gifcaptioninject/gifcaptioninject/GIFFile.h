#pragma once
#include <vector>
#include <cstdint>
#include <optional>

// based on https://www.cs.albany.edu/~sdc/CSI333/Fal07/Lect/L18/Summary.html

#pragma pack(push, 1)
struct GIFColorTableEntry {
    uint8_t r;
    uint8_t g;
    uint8_t b;
};

struct GIFLogicalImageDescriptor {
    uint16_t width;
    uint16_t height;
    /*
    struct {
        uint8_t global_color_table: 1;
        uint8_t color_resolution : 3;
        uint8_t sort_flag : 1;
        uint8_t color_table_size : 3;
    } flags;
    */
    uint8_t flags;
    uint8_t background_color;
    uint8_t aspect_ratio;
};

struct GIFHeader {
    char magic[3];
    char version[3];
    GIFLogicalImageDescriptor image_descriptor;
};

struct GIFLocalImageDescriptorBlock {
    uint8_t separator; // always 0x2C
    uint16_t left;
    uint16_t top;
    uint16_t width;
    uint16_t height;
    /*
    struct {
        uint8_t local_color_table : 1;
        uint8_t interlace : 1;
        uint8_t sort : 1;
        uint8_t reserved : 2;
        uint8_t color_table_size : 3;
    } flags;
    */
    uint8_t flags; // i'm a dumbass and i can't figure out why that bitfield declaration doesn't work
};

struct GIFApplicationExtensionBlockHeader {
    uint8_t introducer; // always 0x21
    uint8_t label;      // always 0xFF
    uint8_t block_size; // always 0x0B
    char identifier[8];
    char authentication_code[3];
};

struct GIFGraphicControlExtensionBlock {
    uint8_t introducer; // always 0x21
    uint8_t label;      // always 0xF9
    uint8_t block_size; // always 0x04
    uint8_t flags;
    uint16_t delay_time;
    uint8_t transparent_color_index;
    uint8_t terminator; // always 0x00
};
#pragma pack(pop)

// these are not actually representative of the in-file structure
struct GIFFrame {
    GIFLocalImageDescriptorBlock image_descriptor;
    std::vector<char> color_table;
    std::vector<char> image_data; // don't care about the contents, only editing frame metadata here
};

class GIFFile {
public:
    GIFFile(std::vector<char> data);
    std::optional<std::vector<char>> Write();
    bool InjectCaption(GIFFile& caption);

    bool parsed = false;
    GIFHeader header = {};
    std::vector<char> global_color_table;
    std::vector<GIFFrame> frames;
    std::vector<char> application_extension;
    std::vector<GIFGraphicControlExtensionBlock> graphic_controls;
private:
    static std::optional<std::vector<char>> ReadSubblocks(char*& cur, char* end);
    static void WriteToVector(std::vector<char>& out, void* data, size_t len);
};