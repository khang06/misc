#include <stdio.h>
#include <vector>

#include "GIFFile.h"

std::vector<char> read_file(const char* filename) {
    FILE* in_file = fopen(filename, "rb");
    if (!in_file) {
        printf("failed to open %s\n", filename);
        exit(1);
    }
    fseek(in_file, 0, SEEK_END);
    size_t in_size = ftell(in_file);
    fseek(in_file, 0, SEEK_SET);
    std::vector<char> in;
    in.resize(in_size);
    fread(in.data(), in.size(), 1, in_file);
    fclose(in_file);
    return in;
}

int main() {
    auto in = read_file("D:\\bruh.gif");
    auto caption_file = read_file("D:\\caption2.gif");

    GIFFile gif(in);
    GIFFile caption(caption_file);
    printf("gif parsed: %s\n", gif.parsed ? "true" : "false");
    printf("caption parsed: %s\n", caption.parsed ? "true" : "false");

    if (!gif.InjectCaption(caption)) {
        printf("failed to inject caption\n");
        return 1;
    }

    FILE* out_file = fopen("rewritten.gif", "wb");
    if (!out_file) {
        printf("failed to open output file\n");
        return 1;
    }
    auto written_gif_res = gif.Write();
    if (!written_gif_res.has_value()) {
        printf("failed to write gif\n");
        return 1;
    }
    auto written_gif = written_gif_res.value();
    fwrite(written_gif.data(), written_gif.size(), 1, out_file);
    fclose(out_file);

    return 0;
}