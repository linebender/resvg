// Copyright 2021 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/**
 * @file resvg.h
 *
 * resvg C API
 */

#ifndef RESVG_H
#define RESVG_H

#include <stdbool.h>
#include <stdint.h>

#define RESVG_MAJOR_VERSION 0
#define RESVG_MINOR_VERSION 45
#define RESVG_PATCH_VERSION 0
#define RESVG_VERSION "0.45.0"

/**
 * @brief List of possible errors.
 */
typedef enum {
    /**
     * Everything is ok.
     */
    RESVG_OK = 0,
    /**
     * Only UTF-8 content are supported.
     */
    RESVG_ERROR_NOT_AN_UTF8_STR,
    /**
     * Failed to open the provided file.
     */
    RESVG_ERROR_FILE_OPEN_FAILED,
    /**
     * Compressed SVG must use the GZip algorithm.
     */
    RESVG_ERROR_MALFORMED_GZIP,
    /**
     * We do not allow SVG with more than 1_000_000 elements for security reasons.
     */
    RESVG_ERROR_ELEMENTS_LIMIT_REACHED,
    /**
     * SVG doesn't have a valid size.
     *
     * Occurs when width and/or height are <= 0.
     *
     * Also occurs if width, height and viewBox are not set.
     */
    RESVG_ERROR_INVALID_SIZE,
    /**
     * Failed to parse an SVG data.
     */
    RESVG_ERROR_PARSING_FAILED,
} resvg_error;

/**
 * @brief A image rendering method.
 */
typedef enum {
    RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY,
    RESVG_IMAGE_RENDERING_OPTIMIZE_SPEED,
} resvg_image_rendering;

/**
 * @brief A shape rendering method.
 */
typedef enum {
    RESVG_SHAPE_RENDERING_OPTIMIZE_SPEED,
    RESVG_SHAPE_RENDERING_CRISP_EDGES,
    RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION,
} resvg_shape_rendering;

/**
 * @brief A text rendering method.
 */
typedef enum {
    RESVG_TEXT_RENDERING_OPTIMIZE_SPEED,
    RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY,
    RESVG_TEXT_RENDERING_GEOMETRIC_PRECISION,
} resvg_text_rendering;

/**
 * @brief An SVG to #resvg_render_tree conversion options.
 *
 * Also, contains a fonts database used during text to path conversion.
 * The database is empty by default.
 */
typedef struct resvg_options resvg_options;

/**
 * @brief An opaque pointer to the rendering tree.
 */
typedef struct resvg_render_tree resvg_render_tree;

/**
 * @brief A 2D transform representation.
 */
typedef struct {
    float a;
    float b;
    float c;
    float d;
    float e;
    float f;
} resvg_transform;

/**
 * @brief A size representation.
 */
typedef struct {
    float width;
    float height;
} resvg_size;

/**
 * @brief A rectangle representation.
 */
typedef struct {
    float x;
    float y;
    float width;
    float height;
} resvg_rect;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

/**
 * @brief Creates an identity transform.
 */
resvg_transform resvg_transform_identity(void);

/**
 * @brief Initializes the library log.
 *
 * Use it if you want to see any warnings.
 *
 * Must be called only once.
 *
 * All warnings will be printed to the `stderr`.
 */
void resvg_init_log(void);

/**
 * @brief Creates a new #resvg_options object.
 *
 * Should be destroyed via #resvg_options_destroy.
 */
resvg_options *resvg_options_create(void);

/**
 * @brief Sets a directory that will be used during relative paths resolving.
 *
 * Expected to be the same as the directory that contains the SVG file,
 * but can be set to any.
 *
 * Must be UTF-8. Can be set to NULL.
 *
 * Default: NULL
 */
void resvg_options_set_resources_dir(resvg_options *opt, const char *path);

/**
 * @brief Sets the target DPI.
 *
 * Impact units conversion.
 *
 * Default: 96
 */
void resvg_options_set_dpi(resvg_options *opt, float dpi);

/**
 * @brief Provides the content of a stylesheet that will be used when resolving CSS attributes.
 *
 * Must be UTF-8. Can be set to NULL.
 *
 * Default: NULL
 */
void resvg_options_set_stylesheet(resvg_options *opt, const char *content);

/**
 * @brief Sets the default font family.
 *
 * Will be used when no `font-family` attribute is set in the SVG.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Default: Times New Roman
 */
void resvg_options_set_font_family(resvg_options *opt, const char *family);

/**
 * @brief Sets the default font size.
 *
 * Will be used when no `font-size` attribute is set in the SVG.
 *
 * Default: 12
 */
void resvg_options_set_font_size(resvg_options *opt, float size);

/**
 * @brief Sets the `serif` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Times New Roman
 */
void resvg_options_set_serif_family(resvg_options *opt, const char *family);

/**
 * @brief Sets the `sans-serif` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Arial
 */
void resvg_options_set_sans_serif_family(resvg_options *opt, const char *family);

/**
 * @brief Sets the `cursive` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Comic Sans MS
 */
void resvg_options_set_cursive_family(resvg_options *opt, const char *family);

/**
 * @brief Sets the `fantasy` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Papyrus on macOS, Impact on other OS'es
 */
void resvg_options_set_fantasy_family(resvg_options *opt, const char *family);

/**
 * @brief Sets the `monospace` font family.
 *
 * Must be UTF-8. NULL is not allowed.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * Default: Courier New
 */
void resvg_options_set_monospace_family(resvg_options *opt, const char *family);

/**
 * @brief Sets a comma-separated list of languages.
 *
 * Will be used to resolve a `systemLanguage` conditional attribute.
 *
 * Example: en,en-US.
 *
 * Must be UTF-8. Can be NULL.
 *
 * Default: en
 */
void resvg_options_set_languages(resvg_options *opt, const char *languages);

/**
 * @brief Sets the default shape rendering method.
 *
 * Will be used when an SVG element's `shape-rendering` property is set to `auto`.
 *
 * Default: `RESVG_SHAPE_RENDERING_GEOMETRIC_PRECISION`
 */
void resvg_options_set_shape_rendering_mode(resvg_options *opt, resvg_shape_rendering mode);

/**
 * @brief Sets the default text rendering method.
 *
 * Will be used when an SVG element's `text-rendering` property is set to `auto`.
 *
 * Default: `RESVG_TEXT_RENDERING_OPTIMIZE_LEGIBILITY`
 */
void resvg_options_set_text_rendering_mode(resvg_options *opt, resvg_text_rendering mode);

/**
 * @brief Sets the default image rendering method.
 *
 * Will be used when an SVG element's `image-rendering` property is set to `auto`.
 *
 * Default: `RESVG_IMAGE_RENDERING_OPTIMIZE_QUALITY`
 */
void resvg_options_set_image_rendering_mode(resvg_options *opt, resvg_image_rendering mode);

/**
 * @brief Loads a font data into the internal fonts database.
 *
 * Prints a warning into the log when the data is not a valid TrueType font.
 *
 * Has no effect when the `text` feature is not enabled.
 */
void resvg_options_load_font_data(resvg_options *opt, const char *data, uintptr_t len);

/**
 * @brief Loads a font file into the internal fonts database.
 *
 * Prints a warning into the log when the data is not a valid TrueType font.
 *
 * Has no effect when the `text` feature is not enabled.
 *
 * @return #resvg_error with RESVG_OK, RESVG_ERROR_NOT_AN_UTF8_STR or RESVG_ERROR_FILE_OPEN_FAILED
 */
int32_t resvg_options_load_font_file(resvg_options *opt, const char *file_path);

/**
 * @brief Loads system fonts into the internal fonts database.
 *
 * This method is very IO intensive.
 *
 * This method should be executed only once per #resvg_options.
 *
 * The system scanning is not perfect, so some fonts may be omitted.
 * Please send a bug report in this case.
 *
 * Prints warnings into the log.
 *
 * Has no effect when the `text` feature is not enabled.
 */
void resvg_options_load_system_fonts(resvg_options *opt);

/**
 * @brief Destroys the #resvg_options.
 */
void resvg_options_destroy(resvg_options *opt);

/**
 * @brief Creates #resvg_render_tree from file.
 *
 * .svg and .svgz files are supported.
 *
 * See #resvg_is_image_empty for details.
 *
 * @param file_path UTF-8 file path.
 * @param opt Rendering options. Must not be NULL.
 * @param tree Parsed render tree. Should be destroyed via #resvg_tree_destroy.
 * @return #resvg_error
 */
int32_t resvg_parse_tree_from_file(const char *file_path,
                                   const resvg_options *opt,
                                   resvg_render_tree **tree);

/**
 * @brief Creates #resvg_render_tree from data.
 *
 * See #resvg_is_image_empty for details.
 *
 * @param data SVG data. Can contain SVG string or gzip compressed data. Must not be NULL.
 * @param len Data length.
 * @param opt Rendering options. Must not be NULL.
 * @param tree Parsed render tree. Should be destroyed via #resvg_tree_destroy.
 * @return #resvg_error
 */
int32_t resvg_parse_tree_from_data(const char *data,
                                   uintptr_t len,
                                   const resvg_options *opt,
                                   resvg_render_tree **tree);

/**
 * @brief Checks that tree has any nodes.
 *
 * @param tree Render tree.
 * @return Returns `true` if tree has no nodes.
 */
bool resvg_is_image_empty(const resvg_render_tree *tree);

/**
 * @brief Returns an image size.
 *
 * The size of an image that is required to render this SVG.
 *
 * Note that elements outside the viewbox will be clipped. This is by design.
 * If you want to render the whole SVG content, use #resvg_get_image_bbox instead.
 *
 * @param tree Render tree.
 * @return Image size.
 */
resvg_size resvg_get_image_size(const resvg_render_tree *tree);

/**
 * @brief Returns an object bounding box.
 *
 * This bounding box does not include objects stroke and filter regions.
 * This is what SVG calls "absolute object bonding box".
 *
 * If you're looking for a "complete" bounding box see #resvg_get_image_bbox
 *
 * @param tree Render tree.
 * @param bbox Image's object bounding box.
 * @return `false` if an image has no elements.
 */
bool resvg_get_object_bbox(const resvg_render_tree *tree, resvg_rect *bbox);

/**
 * @brief Returns an image bounding box.
 *
 * This bounding box contains the maximum SVG dimensions.
 * It's size can be bigger or smaller than #resvg_get_image_size
 * Use it when you want to avoid clipping of elements that are outside the SVG viewbox.
 *
 * @param tree Render tree.
 * @param bbox Image's bounding box.
 * @return `false` if an image has no elements.
 */
bool resvg_get_image_bbox(const resvg_render_tree *tree, resvg_rect *bbox);

/**
 * @brief Returns `true` if a renderable node with such an ID exists.
 *
 * @param tree Render tree.
 * @param id Node's ID. UTF-8 string. Must not be NULL.
 * @return `true` if a node exists.
 * @return `false` if a node doesn't exist or ID isn't a UTF-8 string.
 * @return `false` if a node exists, but not renderable.
 */
bool resvg_node_exists(const resvg_render_tree *tree, const char *id);

/**
 * @brief Returns node's transform by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. UTF-8 string. Must not be NULL.
 * @param transform Node's transform.
 * @return `true` if a node exists.
 * @return `false` if a node doesn't exist or ID isn't a UTF-8 string.
 * @return `false` if a node exists, but not renderable.
 */
bool resvg_get_node_transform(const resvg_render_tree *tree,
                              const char *id,
                              resvg_transform *transform);

/**
 * @brief Returns node's bounding box in canvas coordinates by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. Must not be NULL.
 * @param bbox Node's bounding box.
 * @return `false` if a node with such an ID does not exist
 * @return `false` if ID isn't a UTF-8 string.
 * @return `false` if ID is an empty string
 */
bool resvg_get_node_bbox(const resvg_render_tree *tree, const char *id, resvg_rect *bbox);

/**
 * @brief Returns node's bounding box, including stroke, in canvas coordinates by ID.
 *
 * @param tree Render tree.
 * @param id Node's ID. Must not be NULL.
 * @param bbox Node's bounding box.
 * @return `false` if a node with such an ID does not exist
 * @return `false` if ID isn't a UTF-8 string.
 * @return `false` if ID is an empty string
 */
bool resvg_get_node_stroke_bbox(const resvg_render_tree *tree, const char *id, resvg_rect *bbox);

/**
 * @brief Destroys the #resvg_render_tree.
 */
void resvg_tree_destroy(resvg_render_tree *tree);

/**
 * @brief Renders the #resvg_render_tree onto the pixmap.
 *
 * @param tree A render tree.
 * @param transform A root SVG transform. Can be used to position SVG inside the `pixmap`.
 * @param width Pixmap width.
 * @param height Pixmap height.
 * @param pixmap Pixmap data. Should have width*height*4 size and contain
 *               premultiplied RGBA8888 pixels.
 */
void resvg_render(const resvg_render_tree *tree,
                  resvg_transform transform,
                  uint32_t width,
                  uint32_t height,
                  char *pixmap);

/**
 * @brief Renders a Node by ID onto the image.
 *
 * @param tree A render tree.
 * @param id Node's ID. Must not be NULL.
 * @param transform A root SVG transform. Can be used to position SVG inside the `pixmap`.
 * @param width Pixmap width.
 * @param height Pixmap height.
 * @param pixmap Pixmap data. Should have width*height*4 size and contain
 *               premultiplied RGBA8888 pixels.
 * @return `false` when `id` is not a non-empty UTF-8 string.
 * @return `false` when the selected `id` is not present.
 * @return `false` when an element has a zero bbox.
 */
bool resvg_render_node(const resvg_render_tree *tree,
                       const char *id,
                       resvg_transform transform,
                       uint32_t width,
                       uint32_t height,
                       char *pixmap);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif /* RESVG_H */
