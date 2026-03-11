import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import {
    NbCardModule,
    NbButtonModule,
    NbIconModule,
    NbInputModule,
    NbSelectModule,
    NbSpinnerModule,
    NbDialogModule,
    NbTooltipModule
} from '@nebular/theme';
import { ThemeModule } from '../../@theme/theme.module';
import { ApplicationListComponent } from './application-list/application-list.component';
import { ApplicationDialogComponent } from './application-dialog/application-dialog.component';
import { RouterModule, Routes } from '@angular/router';

const routes: Routes = [
    {
        path: '',
        component: ApplicationListComponent,
    },
];

@NgModule({
    declarations: [
        ApplicationListComponent,
        ApplicationDialogComponent,
    ],
    imports: [
        CommonModule,
        FormsModule,
        ThemeModule,
        NbCardModule,
        NbButtonModule,
        NbIconModule,
        NbInputModule,
        NbSelectModule,
        NbSpinnerModule,
        NbTooltipModule,
        NbDialogModule.forChild(),
        RouterModule.forChild(routes),
    ],
})
export class ApplicationModule { }
