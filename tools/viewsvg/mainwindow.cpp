// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include <QMessageBox>
#include <QTimer>

#include "mainwindow.h"
#include "ui_mainwindow.h"

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
    , ui(new Ui::MainWindow)
{
    ui->setupUi(this);

    SvgView::init();

    ui->cmbBoxSize->setCurrentIndex(1);
    ui->cmbBoxBackground->setCurrentIndex(1);

    ui->svgView->setFitToView(true);
    ui->svgView->setBackground(SvgView::Background::White);

    connect(ui->svgView, &SvgView::loadError, this, [this](const QString &msg){
        QMessageBox::critical(this, "Error", msg);
    });

    QTimer::singleShot(5, this, &MainWindow::onStart);
}

MainWindow::~MainWindow()
{
    delete ui;
}

void MainWindow::onStart()
{
    ui->svgView->setFocus();

    const auto args = QCoreApplication::arguments();
    if (args.size() != 2) {
        return;
    }

    ui->svgView->loadFile(args.at(1));
}

void MainWindow::on_cmbBoxSize_activated(int index)
{
    ui->svgView->setFitToView(index == 1);
}

void MainWindow::on_cmbBoxBackground_activated(int index)
{
    ui->svgView->setBackground(SvgView::Background(index));
}

void MainWindow::on_chBoxDrawBorder_toggled(bool checked)
{
    ui->svgView->setDrawImageBorder(checked);
}
