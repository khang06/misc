LIBNAME = libasound.so.2

all: alsahook.c
	$(CC) alsahook.c -O2 -fPIC -shared -lasound -o libasound.so.2
	cp -f libasound.so.2 libasound.so
	patchelf --replace-needed libasound.so.2 /usr/lib/libasound.so.2 libasound.so.2

clean:
	rm -f libasound.so.2 libasound.so
