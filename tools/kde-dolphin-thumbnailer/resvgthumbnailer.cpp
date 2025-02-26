// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include "resvgthumbnailer.h"

#include <QPainter>

extern "C"
{
    Q_DECL_EXPORT ThumbCreator *new_creator()
    {
        return new ResvgThumbnailer;
    }
}

ResvgThumbnailer::ResvgThumbnailer()
{
    m_opt.loadSystemFonts();
}

bool ResvgThumbnailer::create(const QString& path, int width, int height, QImage& img)
{
    ResvgRenderer renderer(path, m_opt);
    if (!renderer.isValid() || renderer.isEmpty()) {
        return false;
    }

    img = renderer.renderToImage(QSize(width, height) * img.devicePixelRatio());
    return true;
}

ThumbCreator::Flags ResvgThumbnailer::flags() const
{
    return (Flags)(None);
}
