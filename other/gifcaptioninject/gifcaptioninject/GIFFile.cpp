#include <cstdio>
#include <cstdint>
#include <cstring>
#include <optional>
#include "GIFFile.h"

// it's 2021 so i assume the gifs people will use with this don't have weird edge cases
// also assuming sections will always be in the same order
GIFFile::GIFFile(std::vector<char> data) {
    char* cur = data.data();
    char* end = data.data() + data.size();

    // verify that the gif isn't impossibly small before doing anything
    if (cur + sizeof(GIFHeader) > end) {
        printf("gifcaptioninject: not enough space for gif header!\n");
        return;
    }

    // check for a valid trailer
    if (data[data.size() - 1] != 0x3B) {
        printf("gifcaptioninject: gif is truncated!\n");
        return;
    }

    // read the header + logical image descriptor
    memcpy(&header, cur, sizeof(header));
    cur += sizeof(header);
    if (memcmp(header.magic, "GIF", 3) != 0) {
        printf("gifcaptioninject: not a gif file\n");
        return;
    }
    if (memcmp(header.version, "89a", 3) != 0) {
        printf("gifcaptioninject: unsupported gif version\n");
        return;
    }

    // read the global color table if applicable
    if (header.image_descriptor.flags & 0x80) {
        const size_t global_color_table_size = 3LL * (2LL << (header.image_descriptor.flags & 7));
        if (cur + global_color_table_size > end) {
            printf("gifcaptioninject: not enough space for global color table!\n");
            return;
        }
        global_color_table.resize(global_color_table_size);
        memcpy(global_color_table.data(), cur, global_color_table.size());
        cur += global_color_table.size();
    }

    // start actually reading frames and other per-frame metadata
    uint8_t descriptor = 0;
    while (descriptor != 0x3B) {
        if (cur == end) {
            printf("gifcaptioninject: failed to read block descriptor!\n");
            return;
        }
        descriptor = *cur;
        switch (descriptor) {
        case 0x21: {
            // extension block
            if (cur + 1 == end) {
                printf("gifcaptioninject: failed to read extension block label!\n");
                return;
            }
            uint8_t label = *(cur + 1);
            switch (label) {
            case 0xFF: {
                // application extension block
                if (application_extension.size() != 0) {
                    printf("gifcaptioninject: application extension block occurs twice for some reason!\n");
                    return;
                }

                application_extension.resize(sizeof(GIFApplicationExtensionBlockHeader));
                if (cur + application_extension.size() > end) {
                    printf("gifcaptioninject: not enough space for application extension block!\n");
                    return;
                }
                memcpy(application_extension.data(), cur, application_extension.size());
                cur += application_extension.size();

                auto* application_extension_header = (GIFApplicationExtensionBlockHeader*)application_extension.data();
                if (application_extension_header->block_size != 0x0B) {
                    printf("gifcaptioninject: wrong block size for application extension block!\n");
                    return;
                }

                // read the application extension block subblocks
                auto extension_block_res = ReadSubblocks(cur, end);
                if (!extension_block_res.has_value()) {
                    printf("gifcaptioninject: failed to read application extension block subblocks!\n");
                    return;
                }
                application_extension.insert(application_extension.end(), extension_block_res.value().begin(), extension_block_res.value().end());
                break;
            }
            case 0xF9: {
                // graphic control extension
                GIFGraphicControlExtensionBlock graphic_control = {};
                if (cur + sizeof(graphic_control) > end) {
                    printf("gifcaptioninject: not enough space for graphic control extension block!\n");
                    return;
                }
                memcpy(&graphic_control, cur, sizeof(GIFGraphicControlExtensionBlock));
                cur += sizeof(GIFGraphicControlExtensionBlock);

                if (graphic_control.block_size != 0x04) {
                    printf("gifcaptioninject: wrong block size for graphic control extension block!\n");
                    return;
                }

                graphic_controls.push_back(graphic_control);
                break;
            }
            case 0xFE: {
                // comment extension
                if (cur + 2 > end) {
                    printf("gifcaptioninject: not enough space for graphic control extension block!\n");
                    return;
                }
                cur += 2;
                auto comment_block_res = ReadSubblocks(cur, end);
                if (!comment_block_res.has_value()) {
                    printf("gifcaptioninject: failed to read comment extension block subblocks!\n");
                    return;
                }
                // don't actually care about its contents lol
                break;
            }
            default: {
                printf("gifcaptioninject: unhandled extension block label 0x%x!\n", label);
                return;
            }
            }
            break;
        }
        case 0x2C: {
            // image descriptor block
            GIFLocalImageDescriptorBlock image_descriptor = {};
            if (cur + sizeof(image_descriptor) + 1 > end) {
                printf("gifcaptioninject: not enough space for image descriptor block!\n");
                return;
            }
            memcpy(&image_descriptor, cur, sizeof(GIFLocalImageDescriptorBlock));
            cur += sizeof(GIFLocalImageDescriptorBlock);

            // read the local color table if applicable
            std::vector<char> local_color_table;
            if (image_descriptor.flags & 0x80) {
                const size_t local_color_table_size = 3LL * (2LL << (image_descriptor.flags & 7LL));
                if (cur + local_color_table_size > end) {
                    printf("gifcaptioninject: not enough space for local color table!\n");
                    return;
                }
                local_color_table.resize(local_color_table_size);
                memcpy(local_color_table.data(), cur, local_color_table.size());
                cur += local_color_table.size();
            }

            uint8_t lzw_minimum_code_size = *cur++;
            auto subblocks_res = ReadSubblocks(cur, end);
            if (!subblocks_res.has_value()) {
                printf("gifcaptioninject: failed to read image descriptor block subblocks!\n");
                return;
            }

            auto subblocks = subblocks_res.value();
            subblocks.insert(subblocks.begin(), lzw_minimum_code_size);

            GIFFrame frame = {};
            frame.image_descriptor = image_descriptor;
            frame.color_table = local_color_table;
            frame.image_data = subblocks;

            frames.push_back(frame);
            break;
        }
        case 0x3B: {
            // trailer
            break;
        }
        default: {
            printf("gifcaptioninject: unhandled block descriptor 0x%X! file pos %p\n", descriptor, cur - data.data());
            return;
        }
        }
    }

    if (frames.size() != graphic_controls.size()) {
        printf("gifcaptioninject: mismatch between frame count and graphic control count!\n");
        return;
    }

    // we're done here
    parsed = true;
}

std::optional<std::vector<char>> GIFFile::Write() {
    if (!parsed) {
        printf("gifcaptioninject: trying to write gif that wasn't parsed correctly!\n");
        return {};
    }
    if (frames.size() != graphic_controls.size()) {
        printf("gifcaptioninject: frame and graphic control count mismatch during write\n");
        return {};
    }

    std::vector<char> out;
    WriteToVector(out, &header, sizeof(header));
    WriteToVector(out, global_color_table.data(), global_color_table.size());
    WriteToVector(out, application_extension.data(), application_extension.size());
    for (int i = 0; i < frames.size(); i++) {
        WriteToVector(out, &graphic_controls[i], sizeof(GIFGraphicControlExtensionBlock));

        auto& frame = frames[i];
        WriteToVector(out, &frame.image_descriptor, sizeof(GIFLocalImageDescriptorBlock));
        WriteToVector(out, frame.color_table.data(), frame.color_table.size());
        WriteToVector(out, frame.image_data.data(), frame.image_data.size());
    }
    out.push_back(0x3B);

    return out;
}

bool GIFFile::InjectCaption(GIFFile& caption) {
    if (!parsed) {
        printf("gifcaptioninject: trying to inject a caption into a gif that wasn't parsed correctly!\n");
        return false;
    }
    if (!caption.parsed) {
        printf("gifcaptioninject: trying to inject a caption that wasn't parsed correctly!\n");
        return false;
    }
    if (frames.size() == 0) {
        printf("gifcaptioninject: trying to inject a caption into a gif with no frames!\n");
        return false;
    }
    if (caption.frames.size() == 0) {
        printf("gifcaptioninject: trying to inject a caption with no frames!\n");
        return false;
    }

    auto& caption_frame = caption.frames[0];
    auto& caption_descriptor = caption.graphic_controls[0];

    // inject the global color table if there isn't a local one
    if (caption_frame.color_table.size() == 0) {
        caption_frame.color_table = caption.global_color_table;
        caption_frame.image_descriptor.flags |= 0x80;
        caption_frame.image_descriptor.flags = (caption_frame.image_descriptor.flags & ~7) | (caption.header.image_descriptor.flags & 7);
    }

    /*
    // extend the logical height to fit the caption
    // TODO: this will totally overflow
    header.image_descriptor.height += caption_frame.image_descriptor.height;

    // push all of the existing frames downwards
    for (auto& frame : frames)
        frame.image_descriptor.top += caption_frame.image_descriptor.height;
    */

    // extend the logical height to fit the first frame
    // TODO: this will totally overflow
    auto height_diff = caption_frame.image_descriptor.height - header.image_descriptor.height;
    header.image_descriptor.height += height_diff;

    // push all of the existing frames downwards
    for (auto& frame : frames)
        frame.image_descriptor.top += height_diff;

    // delete the first frame
    // i know this is inefficient but who cares
    frames.erase(frames.begin());
    graphic_controls.erase(graphic_controls.begin());

    // inject the new frame at the start
    frames.insert(frames.begin(), caption_frame);
    graphic_controls.insert(graphic_controls.begin(), caption_descriptor);

    // make sure frames won't get disposed
    // TODO: this could totally break shit
    uint8_t flag_mask = (7 << 2);
    for (auto& descriptor : graphic_controls)
        descriptor.flags = (descriptor.flags & ~flag_mask) | (1 << 2);

    return true;
}

std::optional<std::vector<char>> GIFFile::ReadSubblocks(char*& cur, char* end) {
    std::vector<char> out;
    uint8_t size = 0xFF;
    char* old_cur = cur;

    while (size != 0) {
        if (cur + 1 > end)
            return {};
        size = *cur++;
        if (cur + size > end)
            return {};
        cur += size;
    }

    out.resize(cur - old_cur);
    memcpy(out.data(), old_cur, cur - old_cur);
    return out;
}

void GIFFile::WriteToVector(std::vector<char>& out, void* data, size_t len) {
    std::vector<char> temp((char*)data, (char*)data + len);
    out.insert(out.end(), temp.begin(), temp.end());
}