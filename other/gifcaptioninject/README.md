# gifcaptioninject

I wrote this because [esmBot](https://github.com/esmBot/esmBot)'s caption command is horrendously slow. I don't really fault @TheEssem for this because that seems to ImageMagick's fault. Nevertheless, I wanted to make it not so horrendously slow on my self-hosted version and wrote this.

It works by taking the original GIF and a single frame GIF of the caption and injecting the caption as the first frame. Then, it moves the rest of the frames down by the height of the caption and sets their frame clear flags to not overwrite the caption. This doesn't work on every GIF, but it works on enough of them for me not to care.