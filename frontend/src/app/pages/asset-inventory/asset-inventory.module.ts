import { NgModule } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import {
    NbCardModule,
    NbInputModule,
    NbButtonModule,
    NbIconModule,
    NbSpinnerModule,
    NbCheckboxModule,
    NbSelectModule,
    NbTooltipModule,
    NbAutocompleteModule,
} from '@nebular/theme';

import { AssetInventoryRoutingModule } from './asset-inventory-routing.module';
import { AssetInventoryListComponent } from './asset-inventory-list/asset-inventory-list.component';
import { AssetEditDialogComponent } from './asset-edit-dialog/asset-edit-dialog.component';

@NgModule({
    declarations: [
        AssetInventoryListComponent,
        AssetEditDialogComponent,
    ],
    imports: [
        CommonModule,
        FormsModule,
        NbCardModule,
        NbInputModule,
        NbButtonModule,
        NbIconModule,
        NbSpinnerModule,
        NbCheckboxModule,
        NbSelectModule,
        NbTooltipModule,
        NbAutocompleteModule,
        AssetInventoryRoutingModule,
    ],
})
export class AssetInventoryModule { }
