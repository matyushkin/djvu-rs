/*
 * djvulibre_bench.c — minimal libdjvulibre render benchmark.
 *
 * Renders one page of a DjVu file to an in-memory RGB buffer using the
 * ddjvuapi C API, with no file I/O for the output (no PPM write).
 * Reports both total wall time and isolated render-only time.
 *
 * Build:
 *   cc -O2 -o djvulibre_bench djvulibre_bench.c \
 *       $(pkg-config --cflags --libs ddjvuapi)
 *
 * Usage:
 *   ./djvulibre_bench <file.djvu> [page_number_1based] [repeat_count] [target_dpi]
 *
 * target_dpi: output DPI (scales the rendered rectangle).  0 = native DPI.
 */

#include <libdjvu/ddjvuapi.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000.0 + ts.tv_nsec / 1e6;
}

static void handle_messages(ddjvu_context_t *ctx, int wait) {
    const ddjvu_message_t *msg;
    if (wait) ddjvu_message_wait(ctx);
    while ((msg = ddjvu_message_peek(ctx))) {
        if (msg->m_any.tag == DDJVU_ERROR)
            fprintf(stderr, "ddjvu error: %s\n", msg->m_error.message);
        ddjvu_message_pop(ctx);
    }
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <file.djvu> [page] [repeats] [target_dpi]\n", argv[0]);
        return 1;
    }
    const char *path  = argv[1];
    int page_no       = (argc >= 3) ? atoi(argv[2]) - 1 : 0;
    int repeats       = (argc >= 4) ? atoi(argv[3]) : 10;
    int target_dpi    = (argc >= 5) ? atoi(argv[4]) : 0;  /* 0 = native */

    double t_open_start = now_ms();

    ddjvu_context_t  *ctx = ddjvu_context_create("bench");
    ddjvu_document_t *doc = ddjvu_document_create_by_filename(ctx, path, /*cache=*/0);
    if (!doc) { fprintf(stderr, "failed to open %s\n", path); return 1; }

    while (!ddjvu_document_decoding_done(doc))
        handle_messages(ctx, 1);
    handle_messages(ctx, 0);

    ddjvu_page_t *page = ddjvu_page_create_by_pageno(doc, page_no);
    if (!page) { fprintf(stderr, "failed to open page %d\n", page_no); return 1; }

    while (!ddjvu_page_decoding_done(page))
        handle_messages(ctx, 1);
    handle_messages(ctx, 0);

    double t_open_end = now_ms();

    int w   = ddjvu_page_get_width(page);
    int h   = ddjvu_page_get_height(page);
    int dpi = ddjvu_page_get_resolution(page);

    /* Scale output rectangle when target_dpi is requested. */
    int out_dpi = (target_dpi > 0) ? target_dpi : dpi;
    int out_w   = (dpi > 0) ? (int)((double)w * out_dpi / dpi) : w;
    int out_h   = (dpi > 0) ? (int)((double)h * out_dpi / dpi) : h;

    ddjvu_rect_t  prect = {0, 0, (unsigned)w,     (unsigned)h};
    ddjvu_rect_t  rrect = {0, 0, (unsigned)out_w, (unsigned)out_h};
    size_t        stride = (size_t)out_w * 3;
    unsigned char *buf   = malloc(stride * (size_t)out_h);
    if (!buf) { fprintf(stderr, "OOM\n"); return 1; }

    ddjvu_format_t *fmt = ddjvu_format_create(DDJVU_FORMAT_RGB24, 0, NULL);
    ddjvu_format_set_row_order(fmt, 1);

    /* Warm-up */
    ddjvu_page_render(page, DDJVU_RENDER_COLOR, &prect, &rrect, fmt,
                      (unsigned)stride, (char *)buf);

    /* Timed loop — render only (page already decoded) */
    double t_render_start = now_ms();
    for (int i = 0; i < repeats; i++) {
        ddjvu_page_render(page, DDJVU_RENDER_COLOR, &prect, &rrect, fmt,
                          (unsigned)stride, (char *)buf);
    }
    double t_render_end = now_ms();

    unsigned long sum = 0;
    for (size_t i = 0; i < stride * (size_t)out_h; i++) sum += buf[i];

    double open_ms   = t_open_end   - t_open_start;
    double render_ms = (t_render_end - t_render_start) / repeats;

    printf("file   : %s (page %d)\n", path, page_no + 1);
    printf("size   : %dx%d->%dx%d @%ddpi->%ddpi  checksum=%lu\n",
           w, h, out_w, out_h, dpi, out_dpi, sum);
    printf("open+decode : %.2f ms  (parse file + decode page structure)\n", open_ms);
    printf("render_only : %.3f ms  (mean over %d runs, page already in memory)\n",
           render_ms, repeats);

    ddjvu_format_release(fmt);
    free(buf);
    ddjvu_page_release(page);
    ddjvu_document_release(doc);
    ddjvu_context_release(ctx);
    return 0;
}
