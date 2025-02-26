// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#pragma once

#include <QWidget>
#include <QMutex>
#include <QBasicTimer>

#include <ResvgQt.h>

class SvgViewWorker : public QObject
{
    Q_OBJECT

public:
    SvgViewWorker(QObject *parent = nullptr);

    QRect viewBox() const;

public slots:
    QString loadData(const QByteArray &data);
    QString loadFile(const QString &path);
    void render(const QSize &viewSize);

signals:
    void rendered(QImage);

private:
    const float m_dpiRatio;
    mutable QMutex m_mutex;
    ResvgOptions m_opt;
    ResvgRenderer m_renderer;
};

class SvgView : public QWidget
{
    Q_OBJECT

public:
    enum class Background
    {
        None,
        White,
        CheckBoard,
    };

    explicit SvgView(QWidget *parent = nullptr);
    ~SvgView();

    static void init();

    void setFitToView(bool flag);
    void setBackground(Background background);
    void setDrawImageBorder(bool flag);

    void loadData(const QByteArray &data);
    void loadFile(const QString &path);

signals:
    void loadError(QString);

protected:
    void paintEvent(QPaintEvent *);
    void dragEnterEvent(QDragEnterEvent *event);
    void dragMoveEvent(QDragMoveEvent *event);
    void dropEvent(QDropEvent *event);
    void resizeEvent(QResizeEvent *);
    void timerEvent(QTimerEvent *);

private:
    void requestUpdate();
    void afterLoad(const QString &errMsg);
    void drawSpinner(QPainter &p);

private slots:
    void onRendered(const QImage &img);

private:
    const QImage m_checkboardImg;
    SvgViewWorker * const m_worker;
    QTimer * const m_resizeTimer;

    QString m_path;
    float m_dpiRatio = 1.0;
    bool m_isFitToView = true;
    Background m_background = Background::CheckBoard;
    bool m_isDrawImageBorder = false;
    bool m_isHasImage = false;
    QImage m_img;

    QBasicTimer m_timer;
    int m_angle = 0;
};
