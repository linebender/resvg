// Copyright 2017 the Resvg Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#pragma once

#include <QMainWindow>

namespace Ui { class MainWindow; }

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow();

private slots:
    void onStart();
    void on_cmbBoxBackground_activated(int index);
    void on_chBoxDrawBorder_toggled(bool checked);
    void on_cmbBoxSize_activated(int index);

private:
    Ui::MainWindow *ui;
};
