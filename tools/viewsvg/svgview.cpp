// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <QDropEvent>
#include <QElapsedTimer>
#include <QFileInfo>
#include <QMessageBox>
#include <QMimeData>
#include <QPainter>
#include <QThread>
#include <QTimer>

#include "svgview.h"

SvgViewWorker::SvgViewWorker(QObject *parent)
    : QObject(parent)
    , m_dpiRatio(qApp->screens().first()->devicePixelRatio())
{
    m_opt.loadSystemFonts();
}

QRect SvgViewWorker::viewBox() const
{
    QMutexLocker lock(&m_mutex);
    return m_renderer.viewBox();
}

QString SvgViewWorker::loadData(const QByteArray &data)
{
    QMutexLocker lock(&m_mutex);

    m_renderer.load(data, m_opt);
    if (!m_renderer.isValid()) {
        return m_renderer.errorString();
    }

    return QString();
}

QString SvgViewWorker::loadFile(const QString &path)
{
    QMutexLocker lock(&m_mutex);

    m_opt.setResourcesDir(QFileInfo(path).absolutePath());
    m_renderer.load(path, m_opt);
    if (!m_renderer.isValid()) {
        return m_renderer.errorString();
    }

    return QString();
}

void SvgViewWorker::render(const QSize &viewSize)
{
    Q_ASSERT(QThread::currentThread() != qApp->thread());

    QMutexLocker lock(&m_mutex);

    if (m_renderer.isEmpty()) {
        return;
    }

    QElapsedTimer timer;
    timer.start();

    const auto s = m_renderer.defaultSize().scaled(viewSize, Qt::KeepAspectRatio);
    auto img = m_renderer.renderToImage(s * m_dpiRatio);
    img.setDevicePixelRatio(m_dpiRatio);

    qDebug() << QString("Render in %1ms").arg(timer.elapsed());

    emit rendered(img);
}

static QImage genCheckedTexture()
{
    int l = 20;

    QImage pix = QImage(l, l, QImage::Format_RGB32);
    int b = pix.width() / 2.0;
    pix.fill(QColor("#c0c0c0"));

    QPainter p;
    p.begin(&pix);
    p.fillRect(QRect(0,0,b,b), QColor("#808080"));
    p.fillRect(QRect(b,b,b,b), QColor("#808080"));
    p.end();

    return pix;
}

SvgView::SvgView(QWidget *parent)
    : QWidget(parent)
    , m_checkboardImg(genCheckedTexture())
    , m_worker(new SvgViewWorker())
    , m_resizeTimer(new QTimer(this))
{
    setAcceptDrops(true);
    setMinimumSize(10, 10);

    QThread *th = new QThread(this);
    m_worker->moveToThread(th);
    th->start();

    const auto *screen = qApp->screens().first();
    m_dpiRatio = screen->devicePixelRatio();

    connect(m_worker, &SvgViewWorker::rendered, this, &SvgView::onRendered);

    m_resizeTimer->setSingleShot(true);
    connect(m_resizeTimer, &QTimer::timeout, this, &SvgView::requestUpdate);
}

SvgView::~SvgView()
{
    QThread *th = m_worker->thread();
    th->quit();
    th->wait(10000);
    delete m_worker;
}

void SvgView::init()
{
    ResvgRenderer::initLog();
}

void SvgView::setFitToView(bool flag)
{
    m_isFitToView = flag;
    requestUpdate();
}

void SvgView::setBackground(SvgView::Background background)
{
    m_background = background;
    update();
}

void SvgView::setDrawImageBorder(bool flag)
{
    m_isDrawImageBorder = flag;
    update();
}

void SvgView::loadData(const QByteArray &ba)
{
    const QString errMsg = m_worker->loadData(ba);
    afterLoad(errMsg);
}

void SvgView::loadFile(const QString &path)
{
    const QString errMsg = m_worker->loadFile(path);
    afterLoad(errMsg);
}

void SvgView::afterLoad(const QString &errMsg)
{
    m_img = QImage();

    if (errMsg.isEmpty()) {
        m_isHasImage = true;
        requestUpdate();
    } else {
        emit loadError(errMsg);
        m_isHasImage = false;
        update();
    }
}

void SvgView::drawSpinner(QPainter &p)
{
    const int outerRadius = 20;
    const int innerRadius = outerRadius * 0.45;

    const int capsuleHeight = outerRadius - innerRadius;
    const int capsuleWidth  = capsuleHeight * 0.35;
    const int capsuleRadius = capsuleWidth / 2;

    for (int i = 0; i < 12; ++i) {
        QColor color = Qt::black;
        color.setAlphaF(1.0f - (i / 12.0f));
        p.setRenderHint(QPainter::Antialiasing);
        p.setPen(Qt::NoPen);
        p.setBrush(color);
        p.save();
        p.translate(width()/2, height()/2);
        p.rotate(m_angle - i * 30.0f);
        p.drawRoundedRect(-capsuleWidth * 0.5, -(innerRadius + capsuleHeight), capsuleWidth,
                           capsuleHeight, capsuleRadius, capsuleRadius);
        p.restore();
    }
}

void SvgView::paintEvent(QPaintEvent *e)
{
    QPainter p(this);
    const auto r = contentsRect();
    p.setClipRect(r);

    switch (m_background) {
        case Background::None : break;
        case Background::White : {
            p.fillRect(contentsRect(), Qt::white);
        } break;
        case Background::CheckBoard : {
            p.fillRect(contentsRect(), QBrush(m_checkboardImg));
        } break;
    }

    if (m_img.isNull() && !m_timer.isActive()) {
        p.setPen(Qt::black);
        p.drawText(rect(), Qt::AlignCenter, "Drop an SVG image here.");
    } else if (m_timer.isActive()) {
        drawSpinner(p);
    } else {
        const QRect imgRect(0, 0, m_img.width() / m_dpiRatio, m_img.height() / m_dpiRatio);

        p.translate(r.x() + (r.width() - imgRect.width())/ 2,
                    r.y() + (r.height() - imgRect.height()) / 2);

        p.drawImage(0, 0, m_img);

        if (m_isDrawImageBorder) {
            p.setRenderHint(QPainter::Antialiasing, false);
            p.setPen(Qt::green);
            p.setBrush(Qt::NoBrush);
            p.drawRect(imgRect);
        }
    }

    QWidget::paintEvent(e);
}

void SvgView::dragEnterEvent(QDragEnterEvent *event)
{
    event->accept();
}

void SvgView::dragMoveEvent(QDragMoveEvent *event)
{
    event->accept();
}

void SvgView::dropEvent(QDropEvent *event)
{
    const QMimeData *mime = event->mimeData();
    if (!mime->hasUrls()) {
        event->ignore();
        return;
    }

    for (const QUrl &url : mime->urls()) {
        if (!url.isLocalFile()) {
            continue;
        }

        QString path = url.toLocalFile();
        QFileInfo fi = QFileInfo(path);

        if (fi.isFile()) {
            QString suffix = fi.suffix().toLower();
            if (suffix == "svg" || suffix == "svgz") {
                loadFile(path);
            } else {
                QMessageBox::warning(this, tr("Warning"),
                                     tr("You can drop only SVG and SVGZ files."));
            }
        }
    }

    event->acceptProposedAction();
}

void SvgView::resizeEvent(QResizeEvent *)
{
    m_resizeTimer->start(200);
}

void SvgView::timerEvent(QTimerEvent *event)
{
    if (event->timerId() == m_timer.timerId()) {
        m_angle = (m_angle + 30) % 360;
        update();
    } else {
        QWidget::timerEvent(event);
    }
}

void SvgView::requestUpdate()
{
    if (!m_isHasImage) {
        return;
    }

    const auto s = m_isFitToView ? size() : m_worker->viewBox().size();

    if (s * m_dpiRatio == m_img.size()) {
        return;
    }

    m_timer.start(100, this);

    // Run method in the m_worker thread scope.
    QTimer::singleShot(1, m_worker, [=](){
        m_worker->render(s);
    });
}

void SvgView::onRendered(const QImage &img)
{
    m_timer.stop();

    m_img = img;
    update();
}

