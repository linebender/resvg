// Copyright 2018 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use filtra::cpu;
use filtra::types as ft;
use usvg::filter as uf;

pub fn apply(filter: &uf::Filter, ts: tiny_skia::Transform, source: &mut tiny_skia::Pixmap) {
    let filter = convert_filter(filter);
    let (sx, sy) = ts.get_scale();
    let transform = filtra::Transform::new(sx, sy, ts.tx, ts.ty);

    let (width, height) = (source.width(), source.height());
    let pixmap = cpu::PixmapMut::from_data(source.data_mut(), width, height);
    let mut backend = cpu::Backend::new(0);
    filtra::apply(
        &mut backend,
        &filter,
        pixmap,
        transform,
        |root, subregion, rect, data| {
            render_image(root, subregion, transform, rect, data);
        },
    );
}

fn render_image(
    root: &usvg::Group,
    subregion: ft::Rect,
    transform: filtra::Transform,
    rect: filtra::IntRect,
    data: &mut [u8],
) {
    let (width, height) = (rect.width(), rect.height());
    let Some(mut target) = tiny_skia::PixmapMut::from_bytes(data, width, height) else {
        return;
    };

    // usvg places the content relative to the top-left corner of the subregion.
    let ts = tiny_skia::Transform::from_row(
        transform.sx,
        0.0,
        0.0,
        transform.sy,
        subregion.x * transform.sx + transform.tx - rect.x0 as f32,
        subregion.y * transform.sy + transform.ty - rect.y0 as f32,
    );
    let bbox = tiny_skia::IntRect::from_xywh(0, 0, width, height).unwrap();
    let ctx = crate::render::Context {
        max_bbox: bbox,
        needed_bbox: bbox,
    };

    crate::render::render_nodes(root, &ctx, ts, &mut target);
}

fn convert_filter(filter: &uf::Filter) -> ft::Filter<&usvg::Group> {
    let mut converted = ft::Filter::new(convert_rect(filter.rect()));
    let mut results: Vec<&str> = Vec::with_capacity(filter.primitives().len());

    for primitive in filter.primitives() {
        let input = |input: &uf::Input| convert_input(input, &results);
        let (kind, inputs) = match primitive.kind() {
            uf::Kind::Blend(fe) => (
                ft::Primitive::Blend(ft::Blend {
                    mode: convert_blend_mode(fe.mode()),
                    no_composite: false,
                }),
                vec![input(fe.input1()), input(fe.input2())],
            ),
            uf::Kind::ColorMatrix(fe) => (
                ft::Primitive::ColorMatrix(convert_color_matrix(fe.kind())),
                vec![input(fe.input())],
            ),
            uf::Kind::ComponentTransfer(fe) => (
                ft::Primitive::ComponentTransfer(ft::ComponentTransfer {
                    red: convert_transfer_function(fe.func_r()),
                    green: convert_transfer_function(fe.func_g()),
                    blue: convert_transfer_function(fe.func_b()),
                    alpha: convert_transfer_function(fe.func_a()),
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::Composite(fe) => (
                ft::Primitive::Composite(ft::Composite {
                    operator: convert_composite_operator(fe.operator()),
                }),
                vec![input(fe.input1()), input(fe.input2())],
            ),
            uf::Kind::ConvolveMatrix(fe) => (
                ft::Primitive::ConvolveMatrix(convert_convolve_matrix(fe)),
                vec![input(fe.input())],
            ),
            uf::Kind::DiffuseLighting(fe) => (
                ft::Primitive::DiffuseLighting(ft::DiffuseLighting {
                    surface_scale: fe.surface_scale(),
                    diffuse_constant: fe.diffuse_constant(),
                    kernel_unit_length: None,
                    lighting_color: convert_color(fe.lighting_color()),
                    light_source: convert_light_source(fe.light_source()),
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::DisplacementMap(fe) => (
                ft::Primitive::DisplacementMap(ft::DisplacementMap {
                    scale: fe.scale(),
                    x_channel: convert_color_channel(fe.x_channel_selector()),
                    y_channel: convert_color_channel(fe.y_channel_selector()),
                }),
                vec![input(fe.input1()), input(fe.input2())],
            ),
            uf::Kind::DropShadow(fe) => (
                ft::Primitive::DropShadow(ft::DropShadow {
                    dx: fe.dx(),
                    dy: fe.dy(),
                    std_dev_x: fe.std_dev_x().get(),
                    std_dev_y: fe.std_dev_y().get(),
                    color: convert_color(fe.color()),
                    opacity: fe.opacity().get(),
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::Flood(fe) => (
                ft::Primitive::Flood(ft::Flood {
                    color: convert_color(fe.color()),
                    opacity: fe.opacity().get(),
                }),
                Vec::new(),
            ),
            uf::Kind::GaussianBlur(fe) => (
                ft::Primitive::GaussianBlur(ft::GaussianBlur {
                    std_dev_x: fe.std_dev_x().get(),
                    std_dev_y: fe.std_dev_y().get(),
                    edge_mode: ft::EdgeMode::None,
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::Image(fe) => (ft::Primitive::Image(fe.root()), Vec::new()),
            uf::Kind::Merge(fe) => (
                ft::Primitive::Merge(ft::Merge),
                fe.inputs().iter().map(input).collect(),
            ),
            uf::Kind::Morphology(fe) => (
                ft::Primitive::Morphology(ft::Morphology {
                    operator: match fe.operator() {
                        uf::MorphologyOperator::Erode => ft::MorphologyOperator::Erode,
                        uf::MorphologyOperator::Dilate => ft::MorphologyOperator::Dilate,
                    },
                    radius_x: fe.radius_x().get(),
                    radius_y: fe.radius_y().get(),
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::Offset(fe) => (
                ft::Primitive::Offset(ft::Offset {
                    dx: fe.dx(),
                    dy: fe.dy(),
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::SpecularLighting(fe) => (
                ft::Primitive::SpecularLighting(ft::SpecularLighting {
                    surface_scale: fe.surface_scale(),
                    specular_constant: fe.specular_constant(),
                    specular_exponent: fe.specular_exponent(),
                    kernel_unit_length: None,
                    lighting_color: convert_color(fe.lighting_color()),
                    light_source: convert_light_source(fe.light_source()),
                }),
                vec![input(fe.input())],
            ),
            uf::Kind::Tile(fe) => (ft::Primitive::Tile(ft::Tile), vec![input(fe.input())]),
            uf::Kind::Turbulence(fe) => (
                ft::Primitive::Turbulence(ft::Turbulence {
                    base_frequency_x: fe.base_frequency_x().get(),
                    base_frequency_y: fe.base_frequency_y().get(),
                    num_octaves: i32::try_from(fe.num_octaves()).unwrap_or(i32::MAX),
                    seed: fe.seed() as f32,
                    stitch_tiles: fe.stitch_tiles(),
                    kind: match fe.kind() {
                        uf::TurbulenceKind::FractalNoise => ft::TurbulenceType::FractalNoise,
                        uf::TurbulenceKind::Turbulence => ft::TurbulenceType::Turbulence,
                    },
                }),
                Vec::new(),
            ),
        };

        let mut node = ft::Node::new(kind, inputs);
        let rect = primitive.rect();
        node.subregion = ft::Subregion {
            x: Some(rect.x()),
            y: Some(rect.y()),
            width: Some(rect.width()),
            height: Some(rect.height()),
        };
        node.color_interpolation = match primitive.color_interpolation() {
            uf::ColorInterpolation::SRGB => ft::ColorInterpolation::Srgb,
            uf::ColorInterpolation::LinearRGB => ft::ColorInterpolation::LinearRgb,
        };

        converted.push(node);
        results.push(primitive.result());
    }

    converted
}

fn convert_input(input: &uf::Input, results: &[&str]) -> ft::Input {
    match input {
        uf::Input::SourceGraphic => ft::Input::SourceGraphic,
        uf::Input::SourceAlpha => ft::Input::SourceAlpha,
        uf::Input::Reference(name) => results
            .iter()
            .rposition(|result| result == name)
            .map_or(ft::Input::SourceGraphic, ft::Input::Result),
    }
}

fn convert_rect(rect: usvg::NonZeroRect) -> ft::Rect {
    ft::Rect::new(rect.x(), rect.y(), rect.width(), rect.height())
}

fn convert_color(color: usvg::Color) -> ft::Color {
    ft::Color::new(
        f32::from(color.red) / 255.0,
        f32::from(color.green) / 255.0,
        f32::from(color.blue) / 255.0,
        1.0,
    )
}

fn convert_blend_mode(mode: usvg::BlendMode) -> ft::BlendMode {
    match mode {
        usvg::BlendMode::Normal | usvg::BlendMode::PlusLighter => ft::BlendMode::Normal,
        usvg::BlendMode::Multiply => ft::BlendMode::Multiply,
        usvg::BlendMode::Screen => ft::BlendMode::Screen,
        usvg::BlendMode::Overlay => ft::BlendMode::Overlay,
        usvg::BlendMode::Darken => ft::BlendMode::Darken,
        usvg::BlendMode::Lighten => ft::BlendMode::Lighten,
        usvg::BlendMode::ColorDodge => ft::BlendMode::ColorDodge,
        usvg::BlendMode::ColorBurn => ft::BlendMode::ColorBurn,
        usvg::BlendMode::HardLight => ft::BlendMode::HardLight,
        usvg::BlendMode::SoftLight => ft::BlendMode::SoftLight,
        usvg::BlendMode::Difference => ft::BlendMode::Difference,
        usvg::BlendMode::Exclusion => ft::BlendMode::Exclusion,
        usvg::BlendMode::Hue => ft::BlendMode::Hue,
        usvg::BlendMode::Saturation => ft::BlendMode::Saturation,
        usvg::BlendMode::Color => ft::BlendMode::Color,
        usvg::BlendMode::Luminosity => ft::BlendMode::Luminosity,
    }
}

fn convert_color_matrix(kind: &uf::ColorMatrixKind) -> ft::ColorMatrix {
    match kind {
        uf::ColorMatrixKind::Matrix(values) => values
            .as_slice()
            .try_into()
            .map_or_else(|_| ft::ColorMatrix::default(), ft::ColorMatrix::Matrix),
        uf::ColorMatrixKind::Saturate(value) => ft::ColorMatrix::Saturate(value.get()),
        uf::ColorMatrixKind::HueRotate(angle) => ft::ColorMatrix::HueRotate(*angle),
        uf::ColorMatrixKind::LuminanceToAlpha => ft::ColorMatrix::LuminanceToAlpha,
    }
}

fn convert_transfer_function(function: &uf::TransferFunction) -> ft::TransferFunction {
    match function {
        uf::TransferFunction::Identity => ft::TransferFunction::Identity,
        uf::TransferFunction::Table(values) => ft::TransferFunction::Table(values.clone()),
        uf::TransferFunction::Discrete(values) => ft::TransferFunction::Discrete(values.clone()),
        uf::TransferFunction::Linear { slope, intercept } => ft::TransferFunction::Linear {
            slope: *slope,
            intercept: *intercept,
        },
        uf::TransferFunction::Gamma {
            amplitude,
            exponent,
            offset,
        } => ft::TransferFunction::Gamma {
            amplitude: *amplitude,
            exponent: *exponent,
            offset: *offset,
        },
    }
}

fn convert_composite_operator(operator: uf::CompositeOperator) -> ft::CompositeOperator {
    match operator {
        uf::CompositeOperator::Over => ft::CompositeOperator::Over,
        uf::CompositeOperator::In => ft::CompositeOperator::In,
        uf::CompositeOperator::Out => ft::CompositeOperator::Out,
        uf::CompositeOperator::Atop => ft::CompositeOperator::Atop,
        uf::CompositeOperator::Xor => ft::CompositeOperator::Xor,
        uf::CompositeOperator::Lighter => ft::CompositeOperator::Lighter,
        uf::CompositeOperator::Arithmetic { k1, k2, k3, k4 } => {
            ft::CompositeOperator::Arithmetic { k1, k2, k3, k4 }
        }
    }
}

fn convert_convolve_matrix(fe: &uf::ConvolveMatrix) -> ft::ConvolveMatrix {
    let matrix = fe.matrix();

    ft::ConvolveMatrix {
        order_x: matrix.columns(),
        order_y: matrix.rows(),
        kernel: matrix.data().to_vec(),
        divisor: Some(fe.divisor().get()),
        bias: fe.bias(),
        target_x: i32::try_from(matrix.target_x()).ok(),
        target_y: i32::try_from(matrix.target_y()).ok(),
        edge_mode: match fe.edge_mode() {
            uf::EdgeMode::None => ft::EdgeMode::None,
            uf::EdgeMode::Duplicate => ft::EdgeMode::Duplicate,
            uf::EdgeMode::Wrap => ft::EdgeMode::Wrap,
        },
        kernel_unit_length: None,
        preserve_alpha: fe.preserve_alpha(),
    }
}

fn convert_color_channel(channel: uf::ColorChannel) -> ft::ColorChannel {
    match channel {
        uf::ColorChannel::R => ft::ColorChannel::R,
        uf::ColorChannel::G => ft::ColorChannel::G,
        uf::ColorChannel::B => ft::ColorChannel::B,
        uf::ColorChannel::A => ft::ColorChannel::A,
    }
}

fn convert_light_source(source: uf::LightSource) -> ft::LightSource {
    match source {
        uf::LightSource::DistantLight(light) => ft::LightSource::Distant(ft::DistantLight {
            azimuth: light.azimuth,
            elevation: light.elevation,
        }),
        uf::LightSource::PointLight(light) => ft::LightSource::Point(ft::PointLight {
            x: light.x,
            y: light.y,
            z: light.z,
        }),
        uf::LightSource::SpotLight(light) => ft::LightSource::Spot(ft::SpotLight {
            x: light.x,
            y: light.y,
            z: light.z,
            points_at_x: light.points_at_x,
            points_at_y: light.points_at_y,
            points_at_z: light.points_at_z,
            specular_exponent: light.specular_exponent.get(),
            limiting_cone_angle: light.limiting_cone_angle,
        }),
    }
}
