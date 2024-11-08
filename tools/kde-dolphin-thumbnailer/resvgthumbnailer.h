// Copyright 2019 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#ifndef RESVG_THUMBNAILER_H
#define RESVG_THUMBNAILER_H

#include <kio/thumbcreator.h>

#include <ResvgQt.h>

class ResvgThumbnailer : public ThumbCreator
{
public:
    ResvgThumbnailer();
    bool create(const QString& path, int width, int height, QImage& img) override;
    Flags flags() const override;

private:
    ResvgOptions m_opt;
};

#endif
