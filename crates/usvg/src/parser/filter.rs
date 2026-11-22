// Copyright 2022 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A collection of SVG filters.

use std::sync::Arc;

use filtra_types::{
    Blend, BlendMode, ColorChannel, ColorInterpolation, ColorMatrix, ComponentTransfer, Composite,
    CompositeOperator, ConvolveMatrix, DiffuseLighting, DisplacementMap, DistantLight, DropShadow,
    EdgeMode, FilterFunction, Flood, GaussianBlur, Input, LightSource, Merge, Morphology,
    MorphologyOperator, Node, Offset, PointLight, Primitive, SpecularLighting, SpotLight,
    Subregion, Tile, TransferFunction, Turbulence, TurbulenceType,
};
use svgtypes::{AspectRatio, Length, LengthUnit as Unit};

use crate::filter::Filter;
use crate::{
    BlendMode as SvgBlendMode, Group, NonEmptyString, NonZeroRect, Opacity, Size, Transform, Units,
};

use super::OptionLog;
use super::converter;
use super::paint_server::{convert_units, resolve_number};
use super::svgtree::{AId, EId, FromValue, SvgNode};
use super::units::convert_user_length;

impl<'a, 'input: 'a> FromValue<'a, 'input> for ColorInterpolation {
    fn parse(_: SvgNode, _: AId, value: &str) -> Option<Self> {
        match value {
            "sRGB" => Some(ColorInterpolation::Srgb),
            "linearRGB" => Some(ColorInterpolation::LinearRgb),
            _ => None,
        }
    }
}

pub(crate) fn convert(
    node: SvgNode,
    state: &converter::State,
    object_bbox: Option<NonZeroRect>,
    cache: &mut converter::Cache,
) -> Result<Vec<Arc<Filter>>, ()> {
    let value = match node.attribute::<&str>(AId::Filter) {
        Some(v) => v,
        None => return Ok(Vec::new()),
    };

    let mut has_invalid_urls = false;
    let mut filters = Vec::new();

    for func in svgtypes::FilterValueListParser::from(value) {
        let func = match func {
            Ok(v) => v,
            Err(e) => {
                // Skip the whole attribute list on error.
                log::warn!("Failed to parse a filter value cause {}. Skipping.", e);
                return Ok(Vec::new());
            }
        };

        let function = match func {
            svgtypes::FilterValue::Blur(std_dev) => {
                FilterFunction::Blur(convert_user_length(std_dev, node, AId::Dx, state))
            }
            svgtypes::FilterValue::DropShadow {
                color,
                dx,
                dy,
                std_dev,
            } => FilterFunction::DropShadow {
                dx: convert_user_length(dx, node, AId::Dx, state),
                dy: convert_user_length(dy, node, AId::Dy, state),
                std_dev: convert_user_length(std_dev, node, AId::Dx, state),
                color: convert_color(color.unwrap_or_else(|| {
                    node.find_attribute(AId::Color)
                        .unwrap_or_else(svgtypes::Color::black)
                })),
            },
            svgtypes::FilterValue::Brightness(amount) => FilterFunction::Brightness(amount as f32),
            svgtypes::FilterValue::Contrast(amount) => FilterFunction::Contrast(amount as f32),
            svgtypes::FilterValue::Grayscale(amount) => FilterFunction::Grayscale(amount as f32),
            svgtypes::FilterValue::HueRotate(angle) => {
                FilterFunction::HueRotate(angle.to_degrees() as f32)
            }
            svgtypes::FilterValue::Invert(amount) => FilterFunction::Invert(amount as f32),
            svgtypes::FilterValue::Opacity(amount) => FilterFunction::Opacity(amount as f32),
            svgtypes::FilterValue::Sepia(amount) => FilterFunction::Sepia(amount as f32),
            svgtypes::FilterValue::Saturate(amount) => FilterFunction::Saturate(amount as f32),
            svgtypes::FilterValue::Url(url) => {
                if let Some(link) = node.document().element_by_id(url) {
                    if let Ok(res) = convert_url(link, state, object_bbox, cache) {
                        if let Some(f) = res {
                            filters.push(f);
                        }
                    } else {
                        has_invalid_urls = true;
                    }
                } else {
                    has_invalid_urls = true;
                }

                continue;
            }
        };

        let Some(object_bbox) = object_bbox else {
            log::warn!(
                "Filter '{}' has an invalid region. Skipped.",
                node.element_id()
            );
            continue;
        };

        // Filter functions, unlike `filter` elements, do not have a filter region.
        // Instead of the one `from_functions` derives from the content, we currently use a fairly
        // large one. This is far from ideal, but good for now.
        // TODO: Should be fixed eventually.
        let region = match function {
            FilterFunction::DropShadow { .. } | FilterFunction::Blur(_) => {
                NonZeroRect::from_xywh(-0.5, -0.5, 2.0, 2.0).unwrap()
            }
            _ => NonZeroRect::from_xywh(-0.1, -0.1, 1.2, 1.2).unwrap(),
        };
        let region = convert_rect(region.bbox_transform(object_bbox));

        let mut filter = filtra_types::Filter::from_functions(&[function], region);
        filter.region = region;

        filters.push(Arc::new(Filter {
            id: cache.gen_filter_id(),
            filter,
        }));
    }

    // If a `filter` attribute had urls pointing to a missing elements
    // and there are no valid filters at all - this is an error.
    //
    // Note that an invalid url is not an error in general.
    if filters.is_empty() && has_invalid_urls {
        return Err(());
    }

    Ok(filters)
}

fn convert_url(
    node: SvgNode,
    state: &converter::State,
    object_bbox: Option<NonZeroRect>,
    cache: &mut converter::Cache,
) -> Result<Option<Arc<Filter>>, ()> {
    let units = convert_units(node, AId::FilterUnits, Units::ObjectBoundingBox);
    let primitive_units = convert_units(node, AId::PrimitiveUnits, Units::UserSpaceOnUse);

    // Check if this element was already converted.
    //
    // Only `userSpaceOnUse` clipPaths can be shared,
    // because `objectBoundingBox` one will be converted into user one
    // and will become node-specific.
    let cacheable = units == Units::UserSpaceOnUse && primitive_units == Units::UserSpaceOnUse;
    if cacheable {
        if let Some(filter) = cache.filters.get(node.element_id()) {
            return Ok(Some(filter.clone()));
        }
    }

    let rect = NonZeroRect::from_xywh(
        resolve_number(
            node,
            AId::X,
            units,
            state,
            Length::new(-10.0, Unit::Percent),
        ),
        resolve_number(
            node,
            AId::Y,
            units,
            state,
            Length::new(-10.0, Unit::Percent),
        ),
        resolve_number(
            node,
            AId::Width,
            units,
            state,
            Length::new(120.0, Unit::Percent),
        ),
        resolve_number(
            node,
            AId::Height,
            units,
            state,
            Length::new(120.0, Unit::Percent),
        ),
    );

    let mut rect = rect
        .log_none(|| {
            log::warn!(
                "Filter '{}' has an invalid region. Skipped.",
                node.element_id()
            );
        })
        .ok_or(())?;

    if units == Units::ObjectBoundingBox {
        if let Some(object_bbox) = object_bbox {
            rect = rect.bbox_transform(object_bbox);
        } else {
            log::warn!("Filters on zero-sized shapes are not allowed.");
            return Err(());
        }
    }

    let node_with_primitives = match find_filter_with_primitives(node) {
        Some(v) => v,
        None => return Err(()),
    };
    let filter = collect_children(
        &node_with_primitives,
        primitive_units,
        state,
        object_bbox,
        rect,
        cache,
    );
    if filter.nodes().is_empty() {
        return Err(());
    }

    let mut id = NonEmptyString::new(node.element_id().to_string()).ok_or(())?;
    // Generate ID only when we're parsing `objectBoundingBox` filter for the second time.
    if !cacheable && cache.filters.contains_key(id.get()) {
        id = cache.gen_filter_id();
    }
    let id_copy = id.get().to_string();

    let filter = Arc::new(Filter { id, filter });

    cache.filters.insert(id_copy, filter.clone());

    Ok(Some(filter))
}

fn find_filter_with_primitives<'a>(node: SvgNode<'a, 'a>) -> Option<SvgNode<'a, 'a>> {
    for link in node.href_iter() {
        if link.tag_name() != Some(EId::Filter) {
            log::warn!(
                "Filter '{}' cannot reference '{}' via 'xlink:href'.",
                node.element_id(),
                link.tag_name().unwrap()
            );
            return None;
        }

        if link.has_children() {
            return Some(link);
        }
    }

    None
}

fn collect_children(
    filter: &SvgNode,
    units: Units,
    state: &converter::State,
    object_bbox: Option<NonZeroRect>,
    filter_region: NonZeroRect,
    cache: &mut converter::Cache,
) -> filtra_types::Filter<Group> {
    let mut converted = filtra_types::Filter::new(convert_rect(filter_region));
    let mut results = Vec::new();

    let bbox = match (units, object_bbox) {
        // No need to warn. Already checked.
        (Units::ObjectBoundingBox, None) => return converted,
        (Units::ObjectBoundingBox, bbox) => bbox,
        (Units::UserSpaceOnUse, _) => None,
    };
    let scale = bbox.map_or(Size::from_wh(1.0, 1.0).unwrap(), |bbox| bbox.size());

    for child in filter.children() {
        let tag_name = match child.tag_name() {
            Some(v) => v,
            None => continue,
        };

        let subregion = convert_subregion(child, units, state, bbox);
        let mut node = match tag_name {
            EId::FeDropShadow => convert_drop_shadow(child, scale, &results),
            EId::FeGaussianBlur => convert_gaussian_blur(child, scale, &results),
            EId::FeOffset => convert_offset(child, scale, &results),
            EId::FeBlend => convert_blend(child, &results),
            EId::FeFlood => Node::new(Primitive::Flood(convert_flood(child)), []),
            EId::FeComposite => convert_composite(child, &results),
            EId::FeMerge => convert_merge(child, &results),
            EId::FeTile => convert_tile(child, &results),
            EId::FeImage => convert_image(child, subregion, filter_region, state, cache),
            EId::FeComponentTransfer => convert_component_transfer(child, &results),
            EId::FeColorMatrix => convert_color_matrix(child, &results),
            EId::FeConvolveMatrix => convert_convolve_matrix(child, &results),
            EId::FeMorphology => convert_morphology(child, scale, &results),
            EId::FeDisplacementMap => convert_displacement_map(child, scale, &results),
            EId::FeTurbulence => convert_turbulence(child),
            EId::FeDiffuseLighting => convert_diffuse_lighting(child, &results)
                .unwrap_or_else(|| transparent_black(child, &results)),
            EId::FeSpecularLighting => convert_specular_lighting(child, &results)
                .unwrap_or_else(|| transparent_black(child, &results)),
            tag_name => {
                log::warn!("'{}' is not a valid filter primitive. Skipped.", tag_name);
                continue;
            }
        };

        node.subregion = subregion;
        node.color_interpolation = child
            .find_attribute(AId::ColorInterpolationFilters)
            .unwrap_or_default();

        converted.push(node);
        results.push(child.attribute::<&str>(AId::Result));
    }

    converted
}

fn convert_rect(rect: NonZeroRect) -> filtra_types::Rect {
    filtra_types::Rect::from_ltrb(rect.left(), rect.top(), rect.right(), rect.bottom())
}

fn convert_subregion(
    fe: SvgNode,
    units: Units,
    state: &converter::State,
    bbox: Option<NonZeroRect>,
) -> Subregion {
    let length = |aid| fe.try_convert_length(aid, units, state);
    let mut subregion = Subregion {
        x: length(AId::X),
        y: length(AId::Y),
        width: length(AId::Width),
        height: length(AId::Height),
    };

    if let Some(bbox) = bbox {
        subregion.x = subregion.x.map(|x| bbox.x() + x * bbox.width());
        subregion.y = subregion.y.map(|y| bbox.y() + y * bbox.height());
        subregion.width = subregion.width.map(|width| width * bbox.width());
        subregion.height = subregion.height.map(|height| height * bbox.height());
    }

    subregion
}

// A malformed filter primitive produces transparent black. Unlike a transparent `feFlood`, a zero
// color matrix keeps the input that the default subregion is derived from.
fn transparent_black(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let input = resolve_input(fe, AId::In, results);
    Node::new(
        Primitive::ColorMatrix(ColorMatrix::Matrix([0.0; 20])),
        [input],
    )
}

#[inline(never)]
fn resolve_input(node: SvgNode, aid: AId, results: &[Option<&str>]) -> Input {
    // Without `in`, or with an `in` that references an unknown `result`, the input is the result
    // of the previous primitive, or `SourceGraphic` for the first one.
    let previous = || {
        results
            .len()
            .checked_sub(1)
            .map_or(Input::SourceGraphic, Input::Result)
    };

    match node.attribute(aid) {
        Some("SourceGraphic") => Input::SourceGraphic,
        Some("SourceAlpha") => Input::SourceAlpha,
        Some(s @ ("BackgroundImage" | "BackgroundAlpha" | "FillPaint" | "StrokePaint")) => {
            log::warn!("{} filter input isn't supported and not planed.", s);
            Input::SourceGraphic
        }
        Some(name) => results
            .iter()
            .rposition(|result| *result == Some(name))
            .map_or_else(previous, Input::Result),
        None => previous(),
    }
}

fn convert_blend(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let mode = match fe.attribute(AId::Mode).unwrap_or_default() {
        SvgBlendMode::Normal | SvgBlendMode::PlusLighter => BlendMode::Normal,
        SvgBlendMode::Multiply => BlendMode::Multiply,
        SvgBlendMode::Screen => BlendMode::Screen,
        SvgBlendMode::Overlay => BlendMode::Overlay,
        SvgBlendMode::Darken => BlendMode::Darken,
        SvgBlendMode::Lighten => BlendMode::Lighten,
        SvgBlendMode::ColorDodge => BlendMode::ColorDodge,
        SvgBlendMode::ColorBurn => BlendMode::ColorBurn,
        SvgBlendMode::HardLight => BlendMode::HardLight,
        SvgBlendMode::SoftLight => BlendMode::SoftLight,
        SvgBlendMode::Difference => BlendMode::Difference,
        SvgBlendMode::Exclusion => BlendMode::Exclusion,
        SvgBlendMode::Hue => BlendMode::Hue,
        SvgBlendMode::Saturation => BlendMode::Saturation,
        SvgBlendMode::Color => BlendMode::Color,
        SvgBlendMode::Luminosity => BlendMode::Luminosity,
    };
    let input1 = resolve_input(fe, AId::In, results);
    let input2 = resolve_input(fe, AId::In2, results);
    let blend = Blend {
        mode,
        no_composite: false,
    };
    Node::new(Primitive::Blend(blend), [input1, input2])
}

fn convert_color_matrix(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let matrix = convert_color_matrix_kind(fe).unwrap_or_default();
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::ColorMatrix(matrix), [input])
}

fn convert_color_matrix_kind(fe: SvgNode) -> Option<ColorMatrix> {
    match fe.attribute(AId::Type) {
        Some("saturate") => {
            if let Some(list) = fe.attribute::<Vec<f32>>(AId::Values) {
                if !list.is_empty() {
                    return Some(ColorMatrix::Saturate(list[0]));
                } else {
                    return Some(ColorMatrix::Saturate(1.0));
                }
            }
        }
        Some("hueRotate") => {
            if let Some(list) = fe.attribute::<Vec<f32>>(AId::Values) {
                if !list.is_empty() {
                    return Some(ColorMatrix::HueRotate(list[0]));
                } else {
                    return Some(ColorMatrix::HueRotate(0.0));
                }
            }
        }
        Some("luminanceToAlpha") => {
            return Some(ColorMatrix::LuminanceToAlpha);
        }
        _ => {
            // Fallback to `matrix`.
            if let Some(list) = fe.attribute::<Vec<f32>>(AId::Values) {
                if let Ok(matrix) = list.try_into() {
                    return Some(ColorMatrix::Matrix(matrix));
                }
            }
        }
    }

    None
}

fn convert_component_transfer(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let mut transfer = ComponentTransfer::default();

    for child in fe.children().filter(|n| n.is_element()) {
        if let Some(func) = convert_transfer_function(child) {
            match child.tag_name().unwrap() {
                EId::FeFuncR => transfer.red = func,
                EId::FeFuncG => transfer.green = func,
                EId::FeFuncB => transfer.blue = func,
                EId::FeFuncA => transfer.alpha = func,
                _ => {}
            }
        }
    }

    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::ComponentTransfer(transfer), [input])
}

fn convert_transfer_function(node: SvgNode) -> Option<TransferFunction> {
    match node.attribute(AId::Type)? {
        "identity" => Some(TransferFunction::Identity),
        "table" => match node.attribute::<Vec<f32>>(AId::TableValues) {
            Some(values) => Some(TransferFunction::Table(values)),
            None => Some(TransferFunction::Table(Vec::new())),
        },
        "discrete" => match node.attribute::<Vec<f32>>(AId::TableValues) {
            Some(values) => Some(TransferFunction::Discrete(values)),
            None => Some(TransferFunction::Discrete(Vec::new())),
        },
        "linear" => Some(TransferFunction::Linear {
            slope: node.attribute(AId::Slope).unwrap_or(1.0),
            intercept: node.attribute(AId::Intercept).unwrap_or(0.0),
        }),
        "gamma" => Some(TransferFunction::Gamma {
            amplitude: node.attribute(AId::Amplitude).unwrap_or(1.0),
            exponent: node.attribute(AId::Exponent).unwrap_or(1.0),
            offset: node.attribute(AId::Offset).unwrap_or(0.0),
        }),
        _ => None,
    }
}

fn convert_composite(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let operator = match fe.attribute(AId::Operator).unwrap_or("over") {
        "in" => CompositeOperator::In,
        "out" => CompositeOperator::Out,
        "atop" => CompositeOperator::Atop,
        "xor" => CompositeOperator::Xor,
        "lighter" => CompositeOperator::Lighter,
        "arithmetic" => CompositeOperator::Arithmetic {
            k1: fe.attribute(AId::K1).unwrap_or(0.0),
            k2: fe.attribute(AId::K2).unwrap_or(0.0),
            k3: fe.attribute(AId::K3).unwrap_or(0.0),
            k4: fe.attribute(AId::K4).unwrap_or(0.0),
        },
        _ => CompositeOperator::Over,
    };

    let input1 = resolve_input(fe, AId::In, results);
    let input2 = resolve_input(fe, AId::In2, results);
    Node::new(
        Primitive::Composite(Composite { operator }),
        [input1, input2],
    )
}

fn convert_convolve_matrix(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let (order_x, order_y) = match convert_number_optional_number(fe, AId::Order) {
        Some((x, y)) if x >= 1.0 && y >= 1.0 => (x as u32, y as u32),
        _ => (3, 3),
    };

    let edge_mode = match fe.attribute(AId::EdgeMode).unwrap_or("duplicate") {
        "none" => EdgeMode::None,
        "wrap" => EdgeMode::Wrap,
        "mirror" => EdgeMode::Mirror,
        _ => EdgeMode::Duplicate,
    };

    let convolve = ConvolveMatrix {
        order_x,
        order_y,
        kernel: fe.attribute(AId::KernelMatrix).unwrap_or_default(),
        divisor: fe.attribute(AId::Divisor),
        bias: fe.attribute(AId::Bias).unwrap_or(0.0),
        target_x: fe
            .attribute::<f32>(AId::TargetX)
            .map(|target| target as i32),
        target_y: fe
            .attribute::<f32>(AId::TargetY)
            .map(|target| target as i32),
        edge_mode,
        kernel_unit_length: None,
        preserve_alpha: fe.attribute(AId::PreserveAlpha) == Some("true"),
    };
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::ConvolveMatrix(convolve), [input])
}

fn convert_displacement_map(fe: SvgNode, scale: Size, results: &[Option<&str>]) -> Node<Group> {
    let parse_channel = |aid| match fe.attribute(aid).unwrap_or("A") {
        "R" => ColorChannel::R,
        "G" => ColorChannel::G,
        "B" => ColorChannel::B,
        _ => ColorChannel::A,
    };

    // TODO: should probably split scale to scale_x and scale_y
    //       to handle `primitiveUnits=objectBoundingBox` with a non-square bbox.
    let scale = (scale.width() + scale.height()) / 2.0;

    let map = DisplacementMap {
        scale: fe.attribute(AId::Scale).unwrap_or(0.0) * scale,
        x_channel: parse_channel(AId::XChannelSelector),
        y_channel: parse_channel(AId::YChannelSelector),
    };
    let input1 = resolve_input(fe, AId::In, results);
    let input2 = resolve_input(fe, AId::In2, results);
    Node::new(Primitive::DisplacementMap(map), [input1, input2])
}

fn convert_drop_shadow(fe: SvgNode, scale: Size, results: &[Option<&str>]) -> Node<Group> {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, scale, 2.0);
    let Flood { color, opacity } = convert_flood(fe);
    let shadow = DropShadow {
        dx: fe.attribute(AId::Dx).unwrap_or(2.0) * scale.width(),
        dy: fe.attribute(AId::Dy).unwrap_or(2.0) * scale.height(),
        std_dev_x,
        std_dev_y,
        color,
        opacity,
    };
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::DropShadow(shadow), [input])
}

fn convert_flood(fe: SvgNode) -> Flood {
    let color = fe
        .attribute(AId::FloodColor)
        .unwrap_or_else(svgtypes::Color::black);
    let opacity = fe
        .attribute::<Opacity>(AId::FloodOpacity)
        .unwrap_or(Opacity::ONE);

    Flood {
        color: convert_color(color),
        opacity: opacity.get(),
    }
}

fn convert_gaussian_blur(fe: SvgNode, scale: Size, results: &[Option<&str>]) -> Node<Group> {
    let (std_dev_x, std_dev_y) = convert_std_dev_attr(fe, scale, 0.0);
    let blur = GaussianBlur {
        std_dev_x,
        std_dev_y,
        edge_mode: EdgeMode::None,
    };
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::GaussianBlur(blur), [input])
}

fn convert_std_dev_attr(fe: SvgNode, scale: Size, default: f32) -> (f32, f32) {
    let (x, y) =
        convert_number_optional_number(fe, AId::StdDeviation).unwrap_or((default, default));
    (x * scale.width(), y * scale.height())
}

fn convert_number_optional_number(fe: SvgNode, aid: AId) -> Option<(f32, f32)> {
    match *fe.attribute::<Vec<f32>>(aid)? {
        [n] => Some((n, n)),
        [x, y] => Some((x, y)),
        _ => None,
    }
}

fn convert_image(
    fe: SvgNode,
    subregion: Subregion,
    filter_region: NonZeroRect,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Node<Group> {
    match convert_image_inner(fe, subregion, filter_region, state, cache) {
        Some(root) => Node::new(Primitive::Image(root), []),
        None => {
            let flood = Flood {
                color: filtra_types::Color::TRANSPARENT,
                ..Flood::default()
            };
            Node::new(Primitive::Flood(flood), [])
        }
    }
}

fn convert_image_inner(
    fe: SvgNode,
    subregion: Subregion,
    filter_region: NonZeroRect,
    state: &converter::State,
    cache: &mut converter::Cache,
) -> Option<Group> {
    let rendering_mode = fe
        .find_attribute(AId::ImageRendering)
        .unwrap_or(state.opt.image_rendering);

    if let Some(node) = fe.try_attribute::<SvgNode>(AId::Href) {
        let mut state = state.clone();
        state.fe_image_link = true;
        let mut root = Group::empty();
        super::converter::convert_element(node, &state, cache, &mut root);
        return if root.has_children() {
            super::paint_server::update_paint_servers(
                &mut root,
                Transform::default(),
                None,
                None,
                cache,
            );
            // Transfer node id from group's child to the group itself if needed.
            if let Some(crate::Node::Group(g)) = root.children.first_mut() {
                if let Some(child2) = g.children.first_mut() {
                    g.id = child2.id().to_string();
                    match child2 {
                        crate::Node::Group(g2) => g2.id.clear(),
                        crate::Node::Path(path) => path.id = Box::default(),
                        crate::Node::Image(image) => image.id.clear(),
                        crate::Node::Text(text) => text.id.clear(),
                    }
                }
            }

            Some(root)
        } else {
            None
        };
    }

    let href = fe.try_attribute(AId::Href).log_none(|| {
        log::warn!("The 'feImage' element lacks the 'xlink:href' attribute. Skipped.");
    })?;
    let img_data = super::image::get_href_data(href, state)?;
    let actual_size = img_data.actual_size()?;

    let aspect: AspectRatio = fe.attribute(AId::PreserveAspectRatio).unwrap_or_default();
    // The default subregion of `feImage`, which has no inputs, is the filter region.
    let size = Size::from_wh(
        subregion.width.unwrap_or(filter_region.width()),
        subregion.height.unwrap_or(filter_region.height()),
    )?;

    let mut root = Group::empty();
    super::image::convert_inner(
        img_data,
        cache.gen_image_id().take(),
        true,
        rendering_mode,
        aspect,
        actual_size,
        size.to_non_zero_rect(0.0, 0.0),
        cache,
        &mut root,
    );

    Some(root)
}

fn convert_diffuse_lighting(fe: SvgNode, results: &[Option<&str>]) -> Option<Node<Group>> {
    let light_source = convert_light_source(fe)?;
    let lighting = DiffuseLighting {
        surface_scale: fe.attribute(AId::SurfaceScale).unwrap_or(1.0),
        diffuse_constant: fe.attribute(AId::DiffuseConstant).unwrap_or(1.0),
        kernel_unit_length: None,
        lighting_color: convert_lighting_color(fe),
        light_source,
    };
    let input = resolve_input(fe, AId::In, results);
    Some(Node::new(Primitive::DiffuseLighting(lighting), [input]))
}

fn convert_specular_lighting(fe: SvgNode, results: &[Option<&str>]) -> Option<Node<Group>> {
    let light_source = convert_light_source(fe)?;

    let lighting = SpecularLighting {
        surface_scale: fe.attribute(AId::SurfaceScale).unwrap_or(1.0),
        specular_constant: fe.attribute(AId::SpecularConstant).unwrap_or(1.0),
        specular_exponent: fe.attribute(AId::SpecularExponent).unwrap_or(1.0),
        kernel_unit_length: None,
        lighting_color: convert_lighting_color(fe),
        light_source,
    };
    let input = resolve_input(fe, AId::In, results);
    Some(Node::new(Primitive::SpecularLighting(lighting), [input]))
}

#[inline(never)]
fn convert_lighting_color(node: SvgNode) -> filtra_types::Color {
    convert_color(
        node.attribute(AId::LightingColor)
            .unwrap_or_else(svgtypes::Color::white),
    )
}

fn convert_color(color: svgtypes::Color) -> filtra_types::Color {
    let [r, g, b, a] =
        [color.red, color.green, color.blue, color.alpha].map(|c| f32::from(c) / 255.0);
    filtra_types::Color::new(r, g, b, a)
}

#[inline(never)]
fn convert_light_source(parent: SvgNode) -> Option<LightSource> {
    let child = parent.children().find(|n| {
        matches!(
            n.tag_name(),
            Some(EId::FeDistantLight) | Some(EId::FePointLight) | Some(EId::FeSpotLight)
        )
    })?;

    match child.tag_name() {
        Some(EId::FeDistantLight) => Some(LightSource::Distant(DistantLight {
            azimuth: child.attribute(AId::Azimuth).unwrap_or(0.0),
            elevation: child.attribute(AId::Elevation).unwrap_or(0.0),
        })),
        Some(EId::FePointLight) => Some(LightSource::Point(PointLight {
            x: child.attribute(AId::X).unwrap_or(0.0),
            y: child.attribute(AId::Y).unwrap_or(0.0),
            z: child.attribute(AId::Z).unwrap_or(0.0),
        })),
        Some(EId::FeSpotLight) => Some(LightSource::Spot(SpotLight {
            x: child.attribute(AId::X).unwrap_or(0.0),
            y: child.attribute(AId::Y).unwrap_or(0.0),
            z: child.attribute(AId::Z).unwrap_or(0.0),
            points_at_x: child.attribute(AId::PointsAtX).unwrap_or(0.0),
            points_at_y: child.attribute(AId::PointsAtY).unwrap_or(0.0),
            points_at_z: child.attribute(AId::PointsAtZ).unwrap_or(0.0),
            specular_exponent: child.attribute(AId::SpecularExponent).unwrap_or(1.0),
            limiting_cone_angle: child.attribute(AId::LimitingConeAngle),
        })),
        _ => None,
    }
}

fn convert_merge(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let inputs = fe
        .children()
        .map(|child| resolve_input(child, AId::In, results));
    Node::new(Primitive::Merge(Merge), inputs)
}

fn convert_morphology(fe: SvgNode, scale: Size, results: &[Option<&str>]) -> Node<Group> {
    let operator = match fe.attribute(AId::Operator).unwrap_or("erode") {
        "dilate" => MorphologyOperator::Dilate,
        _ => MorphologyOperator::Erode,
    };

    let (radius_x, radius_y) =
        convert_number_optional_number(fe, AId::Radius).unwrap_or((0.0, 0.0));

    let morphology = Morphology {
        operator,
        radius_x: radius_x * scale.width(),
        radius_y: radius_y * scale.height(),
    };
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::Morphology(morphology), [input])
}

fn convert_offset(fe: SvgNode, scale: Size, results: &[Option<&str>]) -> Node<Group> {
    let offset = Offset {
        dx: fe.attribute(AId::Dx).unwrap_or(0.0) * scale.width(),
        dy: fe.attribute(AId::Dy).unwrap_or(0.0) * scale.height(),
    };
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::Offset(offset), [input])
}

fn convert_tile(fe: SvgNode, results: &[Option<&str>]) -> Node<Group> {
    let input = resolve_input(fe, AId::In, results);
    Node::new(Primitive::Tile(Tile), [input])
}

fn convert_turbulence(fe: SvgNode) -> Node<Group> {
    let kind = match fe.attribute(AId::Type).unwrap_or("turbulence") {
        "fractalNoise" => TurbulenceType::FractalNoise,
        _ => TurbulenceType::Turbulence,
    };

    let (base_frequency_x, base_frequency_y) =
        convert_number_optional_number(fe, AId::BaseFrequency).unwrap_or((0.0, 0.0));

    let turbulence = Turbulence {
        base_frequency_x,
        base_frequency_y,
        num_octaves: fe.attribute(AId::NumOctaves).unwrap_or(1.0).round() as i32,
        seed: fe.attribute(AId::Seed).unwrap_or(0.0),
        stitch_tiles: fe.attribute(AId::StitchTiles) == Some("stitch"),
        kind,
    };
    Node::new(Primitive::Turbulence(turbulence), [])
}
