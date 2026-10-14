FROM public.ecr.aws/docker/library/rust:alpine@sha256:0cce0a5e0e8ba67b455257a3a02a1d99005f382748789d6464460028810f1627
RUN apk add --no-cache build-base gfortran clang-dev llvm-dev cmake pkgconf linux-headers \
    curl fontconfig-dev fontconfig-static \
    freetype-dev freetype-static expat-static libpng-static bzip2-static zlib-static brotli-static \
    ttf-dejavu bash file binutils
COPY .sources/OpenBLAS-0.3.30.tar.gz /tmp/openblas.tgz
RUN echo "27342cff518646afb4c2b976d809102e368957974c250a25ccc965e53063c95d  /tmp/openblas.tgz" | sha256sum -c - \
    && tar -xzf /tmp/openblas.tgz -C /tmp \
    && cd /tmp/OpenBLAS-0.3.30 \
    && make -j4 TARGET=GENERIC DYNAMIC_ARCH=0 USE_THREAD=0 NO_SHARED=1 \
    && make PREFIX=/usr TARGET=GENERIC DYNAMIC_ARCH=0 USE_THREAD=0 NO_SHARED=1 install \
    && rm -rf /tmp/OpenBLAS* /tmp/openblas.tgz
COPY .sources/spglib-2.6.0.tar.gz /tmp/spglib.tgz
RUN echo "c65af71136c915352eb82444b165ec83289877eb8e46593033f199801b43dbf7  /tmp/spglib.tgz" | sha256sum -c - \
    && tar -xzf /tmp/spglib.tgz -C /tmp \
    && cmake -S /tmp/spglib-2.6.0 -B /tmp/spglib-build -DSPGLIB_SHARED_LIBS=OFF -DSPGLIB_WITH_Fortran=OFF -DSPGLIB_WITH_TESTS=OFF -DCMAKE_INSTALL_PREFIX=/usr \
    && cmake --build /tmp/spglib-build -j4 && cmake --install /tmp/spglib-build \
    && test -f /usr/lib/libsymspg.a \
    && rm -rf /tmp/spglib*
ENV PKG_CONFIG_ALL_STATIC=1 BINDGEN_EXTRA_CLANG_ARGS=-I/usr/include
WORKDIR /src
